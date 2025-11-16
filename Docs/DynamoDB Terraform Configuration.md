resource "aws_dynamodb_table" "file_metadata" {
name = "FileMetadata"
billing_mode = "PAY_PER_REQUEST"
hash_key = "PK"
range_key = "SK"

# Enable DynamoDB Streams for automatic VIEW_LINK maintenance

stream_enabled = true
stream_view_type = "NEW_AND_OLD_IMAGES"

# Base Table Attributes

attribute { name = "PK" type = "S" }
attribute { name = "SK" type = "S" }

# GSI 1 Attributes

attribute { name = "GSI1-PK" type = "S" }
attribute { name = "GSI1-SK" type = "S" }

# GSI 2 Attributes

attribute { name = "GSI2-PK" type = "S" }
attribute { name = "GSI2-SK" type = "S" }

# GSI 1: ShareAccessIndex (Recipient's "Shared With Me" View)

# Purpose: Allows a recipient to find all grants they have received (PREFIX and FILE grants).

# Query Pattern: GSI1-PK = "ACCESS#<RecipientID>" returns all folders and individual files shared with that user.

# Key Design:

# - PREFIX grants: Access to folder prefix and all files within it (GSI1-SK = "GRANT#<OwnerID>#<Prefix>")

# - FILE grants: Access to single file without folder access (GSI1-SK = "GRANT#<OwnerID>#FILE#<FileID>")

global_secondary_index {
name = "ShareAccessIndex"
hash_key = "GSI1-PK"
range_key = "GSI1-SK"
projection_type = "INCLUDE" # Project attributes needed to display the "Shared With Me" list
non_key_attributes = [
"GrantID", # Unique grant identifier
"GrantType", # "PREFIX" or "FILE" - distinguishes folder grants from file grants
"OwnerID", # Who shared the folder/file
"Permissions", # READ or READ/WRITE
"Prefix", # The folder path that was shared (PREFIX grants only)
"FileID", # The specific file shared (FILE grants only)
"FilePath", # Human-readable path for FILE grants (for UI display)
"CreatedDate" # When the share was created
]
}

# GSI 2: MergedFolderViewIndex (Universal Folder Browsing)

# Purpose: Primary access pattern for ALL folder browsing operations, regardless of ownership.

# Supports S3-style folder derivation using folder marker VIEW_LINKs and file VIEW_LINKs.

# Owners and recipients use the same query pattern to view folder contents.

# Query Pattern: GSI2-PK = "VIEWER#<UserID>#FOLDER#<FolderPrefix>" returns all visible

# folder markers (subfolders) and files, with folders sorted first.

# Sort Key Design: TYPE# prefix enables single-query folder+file browsing:

# - Folder markers: TYPE#FOLDER#<folder_name>#<OwnerID>

# - File VIEW_LINKs: TYPE#FILE#<timestamp>#<media_type>#<file_id>

# Folders naturally sort before files, mimicking S3 bucket browser behavior.

# Timestamp-first design enables pure chronological sorting across all media types and owners.

# Key Benefits: Unified access pattern, no conditional logic, consistent UX, native pagination.

global_secondary_index {
name = "MergedFolderViewIndex"
hash_key = "GSI2-PK"
range_key = "GSI2-SK"
projection_type = "INCLUDE" # Project attributes needed to render folder and file items
non_key_attributes = [
"FileID", # File UUID or "FOLDER#<path>" for folder markers
"OwnerID", # Who owns the file or folder marker
"GrantID", # Which grant authorized this view (for validation)
"CreatedDate", # File/folder creation timestamp
"FileName", # Display name (e.g., "photo.jpg" or "photos/")
"FolderPrefix", # Parent folder path
"MediaType", # MIME type (files only)
"Size" # File size in bytes (files only)
]
}

# Enable point-in-time recovery for data protection

point_in_time_recovery {
enabled = true
}

# TTL attribute for automatic cleanup (optional, for future use)

ttl {
attribute_name = "TTL"
enabled = true
}

tags = {
Name = "FileServiceMetadata"
Environment = "Production"
Service = "FileSharing"
ManagedBy = "Terraform"
}
}

# Lambda function for DynamoDB Streams processing (VIEW_LINK maintenance)

resource "aws_lambda_function" "stream_processor" {
filename = "stream_processor.zip"
function_name = "file-metadata-stream-processor"
role = aws_iam_role.lambda_stream_processor.arn
handler = "bootstrap"
runtime = "provided.al2" # Rust custom runtime
timeout = 300
memory_size = 512

environment {
variables = {
TABLE_NAME = aws_dynamodb_table.file_metadata.name
}
}

tags = {
Name = "StreamProcessor"
Service = "FileSharing"
}
}

# Event source mapping for DynamoDB Streams

# Processes FILE and SHARE_GRANT changes to maintain VIEW_LINKs and folder markers

resource "aws_lambda_event_source_mapping" "stream_trigger" {
event_source_arn = aws_dynamodb_table.file_metadata.stream_arn
function_name = aws_lambda_function.stream_processor.arn
starting_position = "LATEST"
batch_size = 100

# Filter to only process FILE and SHARE_GRANT items

# Stream processor creates VIEW_LINKs (both file and folder marker types)

filter_criteria {
filter {
pattern = jsonencode({
eventName = ["INSERT", "MODIFY", "REMOVE"]
dynamodb = {
NewImage = {
ItemType = {
S = [{ prefix = "FILE" }, { prefix = "SHARE_GRANT" }]
}
}
}
})
}
}
}

# SQS Queue for async VIEW_LINK cleanup

resource "aws_sqs_queue" "view_link_cleanup" {
name = "view-link-cleanup-queue"
visibility_timeout_seconds = 300
message_retention_seconds = 1209600 # 14 days
receive_wait_time_seconds = 20 # Enable long polling

redrive_policy = jsonencode({
deadLetterTargetArn = aws_sqs_queue.view_link_cleanup_dlq.arn
maxReceiveCount = 3
})

tags = {
Name = "ViewLinkCleanupQueue"
Service = "FileSharing"
}
}

# Dead Letter Queue for failed cleanup operations

resource "aws_sqs_queue" "view_link_cleanup_dlq" {
name = "view-link-cleanup-dlq"
message_retention_seconds = 1209600 # 14 days

tags = {
Name = "ViewLinkCleanupDLQ"
Service = "FileSharing"
}
}

# Lambda function for SQS cleanup worker

resource "aws_lambda_function" "cleanup_worker" {
filename = "cleanup_worker.zip"
function_name = "view-link-cleanup-worker"
role = aws_iam_role.lambda_cleanup_worker.arn
handler = "bootstrap"
runtime = "provided.al2" # Rust custom runtime
timeout = 300
memory_size = 512
reserved_concurrent_executions = 5 # Limit concurrency to avoid throttling

environment {
variables = {
TABLE_NAME = aws_dynamodb_table.file_metadata.name
}
}

tags = {
Name = "CleanupWorker"
Service = "FileSharing"
}
}

# Event source mapping for SQS

resource "aws_lambda_event_source_mapping" "cleanup_trigger" {
event_source_arn = aws_sqs_queue.view_link_cleanup.arn
function_name = aws_lambda_function.cleanup_worker.arn
batch_size = 10
}

# IAM Role for Stream Processor Lambda

resource "aws_iam_role" "lambda_stream_processor" {
name = "file-metadata-stream-processor-role"

assume_role_policy = jsonencode({
Version = "2012-10-17"
Statement = [{
Action = "sts:AssumeRole"
Effect = "Allow"
Principal = {
Service = "lambda.amazonaws.com"
}
}]
})
}

# IAM Policy for Stream Processor

resource "aws_iam_role_policy" "stream_processor_policy" {
name = "stream-processor-policy"
role = aws_iam_role.lambda_stream_processor.id

policy = jsonencode({
Version = "2012-10-17"
Statement = [
{
Effect = "Allow"
Action = [
"dynamodb:GetRecords",
"dynamodb:GetShardIterator",
"dynamodb:DescribeStream",
"dynamodb:ListStreams"
]
Resource = aws*dynamodb_table.file_metadata.stream_arn
},
{
Effect = "Allow"
Action = [
"dynamodb:BatchWriteItem",
"dynamodb:PutItem",
"dynamodb:DeleteItem",
"dynamodb:Query"
]
Resource = [
aws_dynamodb_table.file_metadata.arn,
"${aws_dynamodb_table.file_metadata.arn}/index/*"
]
},
{
Effect = "Allow"
Action = [
"logs:CreateLogGroup",
"logs:CreateLogStream",
"logs:PutLogEvents"
]
Resource = "arn:aws:logs:\*:\_:\*"
}
]
})
}

# IAM Role for Cleanup Worker Lambda

resource "aws_iam_role" "lambda_cleanup_worker" {
name = "view-link-cleanup-worker-role"

assume_role_policy = jsonencode({
Version = "2012-10-17"
Statement = [{
Action = "sts:AssumeRole"
Effect = "Allow"
Principal = {
Service = "lambda.amazonaws.com"
}
}]
})
}

# IAM Policy for Cleanup Worker

resource "aws_iam_role_policy" "cleanup_worker_policy" {
name = "cleanup-worker-policy"
role = aws_iam_role.lambda_cleanup_worker.id

policy = jsonencode({
Version = "2012-10-17"
Statement = [
{
Effect = "Allow"
Action = [
"sqs:ReceiveMessage",
"sqs:DeleteMessage",
"sqs:GetQueueAttributes"
]
Resource = aws*sqs_queue.view_link_cleanup.arn
},
{
Effect = "Allow"
Action = [
"dynamodb:BatchWriteItem",
"dynamodb:DeleteItem",
"dynamodb:Query"
]
Resource = [
aws_dynamodb_table.file_metadata.arn,
"${aws_dynamodb_table.file_metadata.arn}/index/*"
]
},
{
Effect = "Allow"
Action = [
"logs:CreateLogGroup",
"logs:CreateLogStream",
"logs:PutLogEvents"
]
Resource = "arn:aws:logs:\*:\_:\*"
}
]
})
}

# CloudWatch Log Groups

resource "aws_cloudwatch_log_group" "stream_processor_logs" {
name = "/aws/lambda/${aws_lambda_function.stream_processor.function_name}"
retention_in_days = 14
}

resource "aws_cloudwatch_log_group" "cleanup_worker_logs" {
name = "/aws/lambda/${aws_lambda_function.cleanup_worker.function_name}"
retention_in_days = 14
}

# Outputs for reference

output "table_name" {
description = "DynamoDB table name"
value = aws_dynamodb_table.file_metadata.name
}

output "table_arn" {
description = "DynamoDB table ARN"
value = aws_dynamodb_table.file_metadata.arn
}

output "stream_arn" {
description = "DynamoDB Streams ARN"
value = aws_dynamodb_table.file_metadata.stream_arn
}

output "cleanup_queue_url" {
description = "SQS cleanup queue URL"
value = aws_sqs_queue.view_link_cleanup.url
}

output "cleanup_queue_arn" {
description = "SQS cleanup queue ARN"
value = aws_sqs_queue.view_link_cleanup.arn
}
