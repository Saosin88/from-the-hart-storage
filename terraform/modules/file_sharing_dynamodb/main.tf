resource "aws_dynamodb_table" "file_metadata" {
  name         = var.table_name
  billing_mode = "PAY_PER_REQUEST"
  hash_key     = "PK"
  range_key    = "SK"

  # Enable DynamoDB Streams for automatic VIEW_LINK maintenance
  stream_enabled   = true
  stream_view_type = "NEW_AND_OLD_IMAGES"

  # Base Table Attributes
  attribute {
    name = "PK"
    type = "S"
  }

  attribute {
    name = "SK"
    type = "S"
  }

  # GSI 1 Attributes
  attribute {
    name = "GSI1-PK"
    type = "S"
  }

  attribute {
    name = "GSI1-SK"
    type = "S"
  }

  # GSI 2 Attributes
  attribute {
    name = "GSI2-PK"
    type = "S"
  }

  attribute {
    name = "GSI2-SK"
    type = "S"
  }

  # GSI 1: ShareAccessIndex (Recipient's "Shared With Me" View)
  # Purpose: Allows a recipient to find all prefix-level grants they have received
  # Query Pattern: GSI1-PK = "ACCESS#<RecipientID>" returns all folders shared with that user
  global_secondary_index {
    name            = "ShareAccessIndex"
    hash_key        = "GSI1-PK"
    range_key       = "GSI1-SK"
    projection_type = "INCLUDE"
    non_key_attributes = [
      "GrantID",
      "OwnerID",
      "Permissions",
      "Prefix",
      "CreatedDate"
    ]
  }

  # GSI 2: MergedFolderViewIndex (Merged, Filterable Folder View)
  # Purpose: Allows a user to see a merged list of contents from multiple shared folders
  # with the same name, with efficient filtering by media type and sorting by creation date
  # Query Pattern: GSI2-PK = "VIEWER#<RecipientID>#FOLDER#<FolderName>"
  global_secondary_index {
    name            = "MergedFolderViewIndex"
    hash_key        = "GSI2-PK"
    range_key       = "GSI2-SK"
    projection_type = "INCLUDE"
    non_key_attributes = [
      "FileID",
      "OwnerID",
      "GrantID",
      "CreatedDate",
      "FileName",
      "MediaType",
      "Size"
    ]
  }

  # Enable point-in-time recovery for data protection
  point_in_time_recovery {
    enabled = var.enable_point_in_time_recovery
  }

  # TTL attribute for automatic cleanup (optional, for future use)
  ttl {
    attribute_name = "TTL"
    enabled        = true
  }

  tags = merge(
    var.tags,
    {
      Name    = "FileServiceMetadata"
      Service = "FileSharing"
    }
  )
}

# SQS Queue for async VIEW_LINK cleanup
resource "aws_sqs_queue" "view_link_cleanup" {
  name                       = "${var.name_prefix}-view-link-cleanup-queue"
  visibility_timeout_seconds = 300
  message_retention_seconds  = 1209600 # 14 days
  receive_wait_time_seconds  = 20      # Enable long polling

  redrive_policy = jsonencode({
    deadLetterTargetArn = aws_sqs_queue.view_link_cleanup_dlq.arn
    maxReceiveCount     = 3
  })

  tags = merge(
    var.tags,
    {
      Name    = "ViewLinkCleanupQueue"
      Service = "FileSharing"
    }
  )
}

# Dead Letter Queue for failed cleanup operations
resource "aws_sqs_queue" "view_link_cleanup_dlq" {
  name                      = "${var.name_prefix}-view-link-cleanup-dlq"
  message_retention_seconds = 1209600 # 14 days

  tags = merge(
    var.tags,
    {
      Name    = "ViewLinkCleanupDLQ"
      Service = "FileSharing"
    }
  )
}

# Lambda function for DynamoDB Streams processing (VIEW_LINK maintenance)
resource "aws_lambda_function" "stream_processor" {
  count = var.create_stream_processor ? 1 : 0

  function_name = "${var.name_prefix}-stream-processor"
  role          = var.lambda_role_arn
  package_type  = "Image"
  image_uri     = var.stream_processor_image_uri
  timeout       = 300
  memory_size   = 512

  environment {
    variables = {
      TABLE_NAME       = aws_dynamodb_table.file_metadata.name
      APP_ENVIRONMENT  = var.environment
      RUST_LOG         = var.log_level
    }
  }

  tags = merge(
    var.tags,
    {
      Name    = "StreamProcessor"
      Service = "FileSharing"
    }
  )
}

# Event source mapping for DynamoDB Streams
resource "aws_lambda_event_source_mapping" "stream_trigger" {
  count = var.create_stream_processor ? 1 : 0

  event_source_arn  = aws_dynamodb_table.file_metadata.stream_arn
  function_name     = aws_lambda_function.stream_processor[0].arn
  starting_position = "LATEST"
  batch_size        = 100

  # Filter to only process FILE and GRANT items
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

# Lambda function for SQS cleanup worker
resource "aws_lambda_function" "cleanup_worker" {
  count = var.create_cleanup_worker ? 1 : 0

  function_name              = "${var.name_prefix}-cleanup-worker"
  role                       = var.lambda_role_arn
  package_type               = "Image"
  image_uri                  = var.cleanup_worker_image_uri
  timeout                    = 300
  memory_size                = 512
  reserved_concurrent_executions = 5 # Limit concurrency to avoid throttling

  environment {
    variables = {
      TABLE_NAME       = aws_dynamodb_table.file_metadata.name
      APP_ENVIRONMENT  = var.environment
      RUST_LOG         = var.log_level
    }
  }

  tags = merge(
    var.tags,
    {
      Name    = "CleanupWorker"
      Service = "FileSharing"
    }
  )
}

# Event source mapping for SQS
resource "aws_lambda_event_source_mapping" "cleanup_trigger" {
  count = var.create_cleanup_worker ? 1 : 0

  event_source_arn = aws_sqs_queue.view_link_cleanup.arn
  function_name    = aws_lambda_function.cleanup_worker[0].arn
  batch_size       = 10
}

# CloudWatch Log Groups
resource "aws_cloudwatch_log_group" "stream_processor_logs" {
  count = var.create_stream_processor ? 1 : 0

  name              = "/aws/lambda/${aws_lambda_function.stream_processor[0].function_name}"
  retention_in_days = var.log_retention_days
}

resource "aws_cloudwatch_log_group" "cleanup_worker_logs" {
  count = var.create_cleanup_worker ? 1 : 0

  name              = "/aws/lambda/${aws_lambda_function.cleanup_worker[0].function_name}"
  retention_in_days = var.log_retention_days
}
