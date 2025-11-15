variable "table_name" {
  description = "Name of the DynamoDB table"
  type        = string
  default     = "FileMetadata"
}

variable "name_prefix" {
  description = "Prefix for resource names"
  type        = string
}

variable "environment" {
  description = "Environment name (dev, staging, prod)"
  type        = string
}

variable "lambda_role_arn" {
  description = "ARN of the IAM role for Lambda functions"
  type        = string
}

variable "stream_processor_image_uri" {
  description = "ECR image URI for the stream processor Lambda"
  type        = string
  default     = ""
}

variable "cleanup_worker_image_uri" {
  description = "ECR image URI for the cleanup worker Lambda"
  type        = string
  default     = ""
}

variable "create_stream_processor" {
  description = "Whether to create the stream processor Lambda function"
  type        = bool
  default     = false
}

variable "create_cleanup_worker" {
  description = "Whether to create the cleanup worker Lambda function"
  type        = bool
  default     = false
}

variable "enable_point_in_time_recovery" {
  description = "Enable point-in-time recovery for DynamoDB table"
  type        = bool
  default     = true
}

variable "log_retention_days" {
  description = "CloudWatch log retention in days"
  type        = number
  default     = 14
}

variable "log_level" {
  description = "Log level for Lambda functions"
  type        = string
  default     = "INFO"
}

variable "tags" {
  description = "Tags to apply to all resources"
  type        = map(string)
  default     = {}
}
