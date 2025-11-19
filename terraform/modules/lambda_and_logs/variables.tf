variable "function_name" {
  description = "Lambda function name"
  type        = string
}

variable "image_uri" {
  description = "ECR image URI for the Lambda function"
  type        = string
}

variable "memory_size" {
  description = "Lambda function memory size in MB"
  type        = number
  default     = 256
}

variable "timeout" {
  description = "Lambda function timeout in seconds"
  type        = number
  default     = 10
}

variable "role_arn" {
  description = "IAM role ARN for the Lambda function"
  type        = string
}

variable "environment_variables" {
  description = "Environment variables for the Lambda function"
  type        = map(string)
  default     = {}
}

variable "log_retention_days" {
  description = "CloudWatch log retention in days"
  type        = number
  default     = 1
}

variable "create_function_url" {
  description = "Whether to create a Lambda function URL"
  type        = bool
  default     = false
}

variable "function_url_authorization_type" {
  description = "Authorization type for function URL (NONE or AWS_IAM)"
  type        = string
  default     = "AWS_IAM"
}

variable "event_source_arn" {
  description = "ARN of the event source (e.g., SQS queue) for event source mapping"
  type        = string
  default     = null
}

variable "event_source_batch_size" {
  description = "Batch size for event source mapping"
  type        = number
  default     = 10
}

variable "event_source_batching_window" {
  description = "Maximum batching window in seconds for event source mapping"
  type        = number
  default     = 30
}

variable "event_source_max_concurrency" {
  description = "Maximum concurrency for event source mapping"
  type        = number
  default     = 2
}
