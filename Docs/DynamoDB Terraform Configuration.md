resource "aws_dynamodb_table" "file_metadata" {
name = "FileMetadata"
billing_mode = "PAY_PER_REQUEST"
hash_key = "PK"
range_key = "SK"

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

# - FILE grants: Access to single file without folder access (GSI1-SK = "GRANT#<OwnerID>#FILE#<ResourceID>")

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
"ResourceID", # The specific file shared (FILE grants only)
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
"ResourceID", # File UUID or "FOLDER#<path>" for folder markers
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

}
}

# S3 Event Processing Infrastructure

# Processes S3 ObjectCreated and ObjectRemoved events to maintain DynamoDB metadata

# SQS Queue for S3 events

resource "aws_sqs_queue" "s3_events" {
name = "file-storage-s3-events"
visibility_timeout_seconds = 300 # 5 minutes for Lambda processing
message_retention_seconds = 1209600 # 14 days
receive_wait_time_seconds = 20 # Enable long polling

redrive_policy = jsonencode({
deadLetterTargetArn = aws_sqs_queue.s3_events_dlq.arn
maxReceiveCount = 3
})

tags = {
Name = "S3EventsQueue"
Service = "FileSharing"
}
}

# Dead Letter Queue for failed S3 event processing

resource "aws_sqs_queue" "s3_events_dlq" {
name = "file-storage-s3-events-dlq"
message_retention_seconds = 1209600 # 14 days

tags = {
Name = "S3EventsDLQ"
Service = "FileSharing"
}
}

# Allow S3 to send messages to SQS

resource "aws_sqs_queue_policy" "s3_events_policy" {
queue_url = aws_sqs_queue.s3_events.id

policy = jsonencode({
Version = "2012-10-17"
Statement = [{
Effect = "Allow"
Principal = {
Service = "s3.amazonaws.com"
}
Action = "sqs:SendMessage"
Resource = aws_sqs_queue.s3_events.arn
Condition = {
ArnEquals = {
"aws:SourceArn" = aws_s3_bucket.file_storage.arn
}
}
}]
})
}

# S3 Bucket with event notifications

resource "aws_s3_bucket_notification" "file_events" {
bucket = aws_s3_bucket.file_storage.id

queue {
queue_arn = aws_sqs_queue.s3_events.arn
events = ["s3:ObjectCreated:*", "s3:ObjectRemoved:*"]
filter_prefix = "" # Process all objects
}
}

# Lambda function for S3 event processing (FILE metadata and VIEW_LINK maintenance)

resource "aws_lambda_function" "s3_event_processor" {
filename = "s3_event_processor.zip"
function_name = "file-storage-s3-event-processor"
role = aws_iam_role.lambda_s3_processor.arn
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
Name = "S3EventProcessor"
Service = "FileSharing"
}
}

# Event source mapping for SQS

resource "aws_lambda_event_source_mapping" "s3_event_trigger" {
event_source_arn = aws_sqs_queue.s3_events.arn
function_name = aws_lambda_function.s3_event_processor.arn
batch_size = 10
}

# IAM Role for S3 Event Processor Lambda

resource "aws_iam_role" "lambda_s3_processor" {
name = "file-storage-s3-processor-role"

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

# IAM Policy for S3 Event Processor

resource "aws_iam_role_policy" "s3_processor_policy" {
name = "s3-processor-policy"
role = aws_iam_role.lambda_s3_processor.id

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
Resource = aws_sqs_queue.s3_events.arn
},
{
Effect = "Allow"
Action = [
"s3:GetObject",
"s3:HeadObject"
]
Resource = "${aws_s3_bucket.file_storage.arn}/*"
      },
      {
        Effect = "Allow"
        Action = [
          "dynamodb:BatchWriteItem",
          "dynamodb:PutItem",
          "dynamodb:DeleteItem",
          "dynamodb:GetItem",
          "dynamodb:Query"
        ]
        Resource = [
          aws_dynamodb_table.file_metadata.arn,
          "${aws_dynamodb_table.file_metadata.arn}/index/_"
]
},
{
Effect = "Allow"
Action = [
"logs:CreateLogGroup",
"logs:CreateLogStream",
"logs:PutLogEvents"
]
Resource = "arn:aws:logs:_:_:_"
}
]
})
}

# CloudWatch Log Group for S3 Event Processor

resource "aws_cloudwatch_log_group" "s3_processor_logs" {
name = "/aws/lambda/${aws_lambda_function.s3_event_processor.function_name}"
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

output "s3_events_queue_url" {
description = "SQS S3 events queue URL"
value = aws_sqs_queue.s3_events.url
}

output "s3_events_queue_arn" {
description = "SQS S3 events queue ARN"
value = aws_sqs_queue.s3_events.arn
}
