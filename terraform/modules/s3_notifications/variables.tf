variable "bucket_id" {
  description = "S3 bucket name (id) to attach notifications to"
  type        = string
}
variable "bucket_arn" {
  description = "S3 bucket ARN (for queue policy condition)"
  type        = string
}
variable "name_prefix" {
  description = "Name prefix for queues"
  type        = string
  default     = "from-the-hart"
}
variable "max_receive_count" {
  type    = number
  default = 5
}