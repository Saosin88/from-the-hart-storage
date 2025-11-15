module "from_the_hart_storage" {
  source = "../modules/s3_cloudfront_secure_storage"

  domain_name                        = "dev-storage.fromthehart.tech"
  acm_certificate_arn                = data.terraform_remote_state.shared.outputs.acm_certificate_arn
  ssm_parameter_name_for_private_key = "/from-the-hart-tech-storage/dev/cloudfront-private-key"

  tags = {
    Domain      = "tech"
    Project     = "from-the-hart-storage"
    Environment = "dev"
    Terraform   = "true"
  }
}

module "from_the_hart_storage_notifications" {
  source      = "../modules/s3_notifications"
  bucket_id   = module.from_the_hart_storage.s3_bucket_name
  bucket_arn  = module.from_the_hart_storage.s3_bucket_arn
  name_prefix = "from-the-hart-storage-dev"
}

# File Sharing Service DynamoDB Table and Infrastructure
module "file_sharing" {
  source = "../modules/file_sharing_dynamodb"

  table_name   = "FileMetadata-dev"
  name_prefix  = "from-the-hart-storage-dev"
  environment  = "dev"
  lambda_role_arn = data.terraform_remote_state.shared.outputs.from_the_hart_lambda_role_arn

  # Lambda functions will be added when stream processor and cleanup worker are implemented
  create_stream_processor = false
  create_cleanup_worker   = false

  enable_point_in_time_recovery = true
  log_retention_days            = 14
  log_level                     = "INFO"

  tags = {
    Domain      = "tech"
    Project     = "from-the-hart-storage"
    Environment = "dev"
    Terraform   = "true"
  }
}

variable "lambda_image_uri_http" {
  description = "ECR image URI for the HTTP Lambda function"
  type        = string
}

variable "lambda_image_uri_sqs" {
  description = "ECR image URI for the SQS Lambda function"
  type        = string
}

resource "aws_lambda_function" "from_the_hart_storage_http_worker" {
  function_name = "from-the-hart-storage-http-worker-dev"
  package_type  = "Image"
  image_uri     = var.lambda_image_uri_http
  memory_size   = 256
  timeout       = 10
  role          = data.terraform_remote_state.shared.outputs.from_the_hart_lambda_role_arn

  architectures = ["x86_64"]

  environment {
    variables = {
      APP_ENVIRONMENT       = "dev"
      RUST_LOG              = "INFO"
      APP_TIMEZONE          = "Africa/Johannesburg"
      DYNAMODB_TABLE_NAME   = module.file_sharing.table_name
      SQS_CLEANUP_QUEUE_URL = module.file_sharing.cleanup_queue_url
    }
  }
}

resource "aws_lambda_function_url" "from_the_hart_storage_function_url" {
  function_name      = aws_lambda_function.from_the_hart_storage_http_worker.function_name
  authorization_type = "AWS_IAM"
  depends_on = [
    aws_lambda_function.from_the_hart_storage_http_worker,
  ]
}

resource "aws_cloudwatch_log_group" "from_the_hart_storage_log_group" {
  name = "/aws/lambda/${aws_lambda_function.from_the_hart_storage_http_worker.function_name}"

  retention_in_days = 1
}

resource "aws_lambda_function" "from_the_hart_storage_sqs_worker" {
  function_name = "from-the-hart-storage-sqs-worker-dev"
  package_type  = "Image"
  image_uri     = var.lambda_image_uri_sqs
  memory_size   = 512
  timeout       = 300
  role          = data.terraform_remote_state.shared.outputs.from_the_hart_lambda_role_arn

  architectures = ["x86_64"]

  environment {
    variables = {
      APP_ENVIRONMENT       = "dev"
      RUST_LOG              = "INFO"
      APP_TIMEZONE          = "Africa/Johannesburg"
      DYNAMODB_TABLE_NAME   = module.file_sharing.table_name
      SQS_CLEANUP_QUEUE_URL = module.file_sharing.cleanup_queue_url
    }
  }
}

resource "aws_lambda_event_source_mapping" "sqs_to_lambda" {
  event_source_arn = module.from_the_hart_storage_notifications.queue_arn
  function_name    = aws_lambda_function.from_the_hart_storage_sqs_worker.arn

  batch_size                         = 10
  maximum_batching_window_in_seconds = 30

  function_response_types = ["ReportBatchItemFailures"]

  scaling_config {
    maximum_concurrency = 2
  }
}

resource "aws_cloudwatch_log_group" "from_the_hart_storage_sqs_worker_log_group" {
  name              = "/aws/lambda/${aws_lambda_function.from_the_hart_storage_sqs_worker.function_name}"
  retention_in_days = 1
}