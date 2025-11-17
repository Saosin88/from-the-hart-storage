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
Resource = aws*sqs_queue.s3_events.arn
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
Resource = "arn:aws:logs:_:_:\_"
}
]
})
}

# CloudWatch Log Group for S3 Event Processor

resource "aws_cloudwatch_log_group" "s3_processor_logs" {
name = "/aws/lambda/${aws_lambda_function.s3_event_processor.function_name}"
retention_in_days = 14
}

# CloudFront Distribution with Origin Access Control (OAC)

# CloudFront Public Key (for signed URL verification)

resource "aws_cloudfront_public_key" "storage" {
comment = "Public key for storage service signed URLs"
encoded_key = file("${path.module}/cloudfront_public_key.pem")
name = "storage-public-key"
}

# CloudFront Key Group (associates public key with distribution)

resource "aws_cloudfront_key_group" "storage" {
comment = "Key group for storage service signed URLs"
items = [aws_cloudfront_public_key.storage.id]
name = "storage-key-group"
}

# Origin Access Control (OAC) for S3

resource "aws_cloudfront_origin_access_control" "storage" {
name = "storage-oac"
description = "OAC for private S3 bucket access"
origin_access_control_origin_type = "s3"
signing_behavior = "always"
signing_protocol = "sigv4"
}

# CloudFront Distribution

resource "aws_cloudfront_distribution" "storage" {
enabled = true
is_ipv6_enabled = true
comment = "File storage service distribution"
default_root_object = ""
aliases = ["dev-storage.fromthehart.tech"] # Or "storage.fromthehart.tech" in prod

# Origin: S3 Bucket

origin {
domain_name = aws_s3_bucket.file_storage.bucket_regional_domain_name
origin_id = "S3-${aws_s3_bucket.file_storage.id}"
origin_access_control_id = aws_cloudfront_origin_access_control.storage.id
}

# Default Cache Behavior (requires signed URLs)

default_cache_behavior {
allowed_methods = ["GET", "HEAD", "OPTIONS", "PUT", "POST", "PATCH", "DELETE"]
cached_methods = ["GET", "HEAD"]
target_origin_id = "S3-${aws_s3_bucket.file_storage.id}"
viewer_protocol_policy = "redirect-to-https"
compress = true

# Trusted Key Groups for signature verification

trusted_key_groups = [aws_cloudfront_key_group.storage.id]

# Disable caching (files are private, cache would bypass permission checks)

cache_policy_id = data.aws_cloudfront_cache_policy.caching_disabled.id
origin_request_policy_id = data.aws_cloudfront_origin_request_policy.all_viewer.id
}

# Public Cache Behavior (no signed URLs required) - for future use

ordered_cache_behavior {
path_pattern = "/public/\*"
allowed_methods = ["GET", "HEAD", "OPTIONS"]
cached_methods = ["GET", "HEAD"]
target_origin_id = "S3-${aws_s3_bucket.file_storage.id}"
viewer_protocol_policy = "redirect-to-https"
compress = true

# No trusted key groups = no signature required

cache_policy_id = data.aws_cloudfront_cache_policy.caching_optimized.id
}

# SSL/TLS Certificate

viewer_certificate {
acm_certificate_arn = var.acm_certificate_arn
ssl_support_method = "sni-only"
minimum_protocol_version = "TLSv1.2_2021"
}

# Geo Restrictions (optional)

restrictions {
geo_restriction {
restriction_type = "none"
}
}

# Logging Configuration (optional)

logging_config {
include_cookies = false
bucket = aws_s3_bucket.cloudfront_logs.bucket_domain_name
prefix = "storage/"
}

tags = {
Name = "StorageDistribution"
Environment = var.environment
Service = "FileSharing"
}
}

# Data Sources for CloudFront Policies

data "aws_cloudfront_cache_policy" "caching_disabled" {
name = "Managed-CachingDisabled"
}

data "aws_cloudfront_cache_policy" "caching_optimized" {
name = "Managed-CachingOptimized"
}

data "aws_cloudfront_origin_request_policy" "all_viewer" {
name = "Managed-AllViewer"
}

# S3 Bucket Policy for OAC Access

resource "aws_s3_bucket_policy" "cloudfront_oac" {
bucket = aws_s3_bucket.file_storage.id

policy = jsonencode({
Version = "2012-10-17"
Statement = [
{
Sid = "AllowCloudFrontServicePrincipal"
Effect = "Allow"
Principal = {
Service = "cloudfront.amazonaws.com"
}
Action = [
"s3:GetObject",
"s3:PutObject",
"s3:DeleteObject"
]
Resource = "${aws_s3_bucket.file_storage.arn}/\*"
Condition = {
StringEquals = {
"AWS:SourceArn" = aws_cloudfront_distribution.storage.arn
}
}
}
]
})
}

# SSM Parameter for CloudFront Private Key

resource "aws_ssm_parameter" "cloudfront_private_key" {
name = "/cloudfront/storage/private-key"
description = "CloudFront private key for signed URL generation"
type = "SecureString"
value = file("${path.module}/cloudfront_private_key.pem")

# KMS key for encryption (optional, uses default AWS managed key if omitted)

key_id = var.kms_key_id

tags = {
Name = "CloudFrontPrivateKey"
Environment = var.environment
Service = "FileSharing"
}

lifecycle {
ignore_changes = [value] # Prevent accidental overwrites
}
}

# IAM Policy for API Lambda to Access SSM Parameter

resource "aws_iam_policy" "api_lambda_ssm_access" {
name = "api-lambda-ssm-cloudfront-key-access"
description = "Allow API Lambda to read CloudFront private key from SSM"

policy = jsonencode({
Version = "2012-10-17"
Statement = [
{
Effect = "Allow"
Action = [
"ssm:GetParameter"
]
Resource = aws_ssm_parameter.cloudfront_private_key.arn
},
{
Effect = "Allow"
Action = [
"kms:Decrypt"
]
Resource = var.kms_key_id # KMS key used to encrypt SSM parameter
}
]
})
}

# Attach SSM Policy to API Lambda Role

resource "aws_iam_role_policy_attachment" "api_lambda_ssm" {
role = aws_iam_role.api_lambda.name
policy_arn = aws_iam_policy.api_lambda_ssm_access.arn
}

# CloudFront Logs S3 Bucket (optional)

resource "aws_s3_bucket" "cloudfront_logs" {
bucket = "storage-cloudfront-logs-${var.environment}"

tags = {
Name = "CloudFrontLogs"
Environment = var.environment
Service = "FileSharing"
}
}

resource "aws_s3_bucket_lifecycle_configuration" "cloudfront_logs" {
bucket = aws_s3_bucket.cloudfront_logs.id

rule {
id = "expire-old-logs"
status = "Enabled"

expiration {
days = 90 # Retain logs for 90 days
}
}
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

output "cloudfront_distribution_id" {
description = "CloudFront distribution ID"
value = aws_cloudfront_distribution.storage.id
}

output "cloudfront_distribution_domain" {
description = "CloudFront distribution domain name"
value = aws_cloudfront_distribution.storage.domain_name
}

output "cloudfront_distribution_arn" {
description = "CloudFront distribution ARN"
value = aws_cloudfront_distribution.storage.arn
}

output "cloudfront_public_key_id" {
description = "CloudFront public key ID (use in Key-Pair-Id query parameter)"
value = aws_cloudfront_public_key.storage.id
}

output "ssm_private_key_parameter_name" {
description = "SSM parameter name for CloudFront private key"
value = aws_ssm_parameter.cloudfront_private_key.name
}
