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

module "http_worker" {
  source = "../modules/lambda_and_logs"

  function_name        = "from-the-hart-storage-http-worker-dev"
  image_uri            = var.lambda_image_uri_http
  memory_size          = 256
  timeout              = 10
  role_arn             = data.terraform_remote_state.shared.outputs.from_the_hart_lambda_role_arn
  create_function_url  = true
  log_retention_days   = 1

  environment_variables = {
    APP_ENVIRONMENT     = "dev"
    RUST_LOG            = "INFO"
    APP_TIMEZONE        = "Africa/Johannesburg"
    APP_DYNAMODB_TABLE  = module.dynamodb.table_name
  }
}

module "sqs_worker" {
  source = "../modules/lambda_and_logs"

  function_name      = "from-the-hart-storage-sqs-worker-dev"
  image_uri          = var.lambda_image_uri_sqs
  memory_size        = 512
  timeout            = 300
  role_arn           = data.terraform_remote_state.shared.outputs.from_the_hart_lambda_role_arn
  log_retention_days = 1

  environment_variables = {
    APP_ENVIRONMENT    = "dev"
    RUST_LOG           = "INFO"
    APP_TIMEZONE       = "Africa/Johannesburg"
    APP_DYNAMODB_TABLE = module.dynamodb.table_name
  }

  event_source_arn              = module.from_the_hart_storage_notifications.queue_arn
  event_source_batch_size       = 10
  event_source_batching_window  = 30
  event_source_max_concurrency  = 2
}

# Moved blocks to preserve existing resources
moved {
  from = aws_lambda_function.from_the_hart_storage_http_worker
  to   = module.http_worker.aws_lambda_function.this
}

moved {
  from = aws_lambda_function_url.from_the_hart_storage_function_url
  to   = module.http_worker.aws_lambda_function_url.this[0]
}

moved {
  from = aws_cloudwatch_log_group.from_the_hart_storage_log_group
  to   = module.http_worker.aws_cloudwatch_log_group.this
}

moved {
  from = aws_lambda_function.from_the_hart_storage_sqs_worker
  to   = module.sqs_worker.aws_lambda_function.this
}

moved {
  from = aws_lambda_event_source_mapping.sqs_to_lambda
  to   = module.sqs_worker.aws_lambda_event_source_mapping.this[0]
}

moved {
  from = aws_cloudwatch_log_group.from_the_hart_storage_sqs_worker_log_group
  to   = module.sqs_worker.aws_cloudwatch_log_group.this
}


module "dynamodb" {
  source = "../modules/dynamodb"

  name = "from-the-hart-storage-dev"

  tags = {
    Domain      = "tech"
    Project     = "from-the-hart-storage"
    Environment = "dev"
    Terraform   = "true"
  }
}

