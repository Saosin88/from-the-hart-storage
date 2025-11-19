resource "aws_lambda_function" "this" {
  function_name = var.function_name
  package_type  = "Image"
  image_uri     = var.image_uri
  memory_size   = var.memory_size
  timeout       = var.timeout
  role          = var.role_arn

  architectures = ["x86_64"]

  environment {
    variables = var.environment_variables
  }
}

resource "aws_lambda_function_url" "this" {
  count = var.create_function_url ? 1 : 0

  function_name      = aws_lambda_function.this.function_name
  authorization_type = var.function_url_authorization_type

  depends_on = [
    aws_lambda_function.this,
  ]
}

resource "aws_cloudwatch_log_group" "this" {
  name              = "/aws/lambda/${aws_lambda_function.this.function_name}"
  retention_in_days = var.log_retention_days
}

resource "aws_lambda_event_source_mapping" "this" {
  count = var.event_source_arn != null ? 1 : 0

  event_source_arn = var.event_source_arn
  function_name    = aws_lambda_function.this.arn

  batch_size                         = var.event_source_batch_size
  maximum_batching_window_in_seconds = var.event_source_batching_window

  function_response_types = ["ReportBatchItemFailures"]

  scaling_config {
    maximum_concurrency = var.event_source_max_concurrency
  }
}
