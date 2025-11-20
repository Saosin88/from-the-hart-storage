variable "name" {
  description = "DynamoDB table name"
  type        = string
}

variable "gsi2_name" {
  description = "Name for the GSI2 (view-link-index)"
  type        = string
  default     = "view-link-index"
}

variable "tags" {
  description = "Tags to apply to the DynamoDB table"
  type        = map(string)
  default     = {}
}
