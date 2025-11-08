variable "acm_certificate_arn" {
  description = "ARN of the ACM certificate for CloudFront"
  type        = string
}

variable "domain_name" {
  description = "Domain name for CloudFront aliases and S3 bucket"
  type        = string
}

variable "ssm_parameter_name_for_private_key" {
  description = "The name of the SSM Parameter Store parameter to store the private key."
  type        = string
}

variable "tags" {
  description = "Tags to apply to all resources"
  type        = map(string)
}