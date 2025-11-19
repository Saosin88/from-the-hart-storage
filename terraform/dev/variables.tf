variable "lambda_image_uri_http" {
  description = "ECR image URI for the HTTP Lambda function"
  type        = string
}

variable "lambda_image_uri_sqs" {
  description = "ECR image URI for the SQS Lambda function"
  type        = string
}