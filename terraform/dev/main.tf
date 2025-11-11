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

variable "lambda_image_uri" {
  description = "ECR image URI for the Lambda function"
  type        = string
}

resource "aws_lambda_function" "from_the_hart_storage_lambda_function" {
  function_name = "from-the-hart-storage-dev"
  package_type  = "Image"
  image_uri     = var.lambda_image_uri
  memory_size   = 512
  timeout       = 90
  role          = data.terraform_remote_state.shared.outputs.from_the_hart_lambda_role_arn

  architectures = ["x86_64"]

  environment {
    variables = {
      APP_ENVIRONMENT = "dev"
      RUST_LOG        = "info"
    }
  }
}

resource "aws_lambda_function_url" "from_the_hart_storage_function_url" {
  function_name      = aws_lambda_function.from_the_hart_storage_lambda_function.function_name
  authorization_type = "AWS_IAM"
  depends_on = [
    aws_lambda_function.from_the_hart_storage_lambda_function,
  ]
}

resource "aws_cloudwatch_log_group" "from_the_hart_storage_log_group" {
  name = "/aws/lambda/${aws_lambda_function.from_the_hart_storage_lambda_function.function_name}"

  retention_in_days = 1
}