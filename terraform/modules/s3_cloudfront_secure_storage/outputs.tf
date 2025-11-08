output "cloudfront_distribution_id" {
  description = "The ID of the CloudFront distribution."
  value       = aws_cloudfront_distribution.storage.id
}

output "cloudfront_distribution_domain_name" {
  description = "The domain name of the CloudFront distribution."
  value       = aws_cloudfront_distribution.storage.domain_name
}

output "s3_bucket_name" {
  description = "The name of the S3 bucket."
  value       = aws_s3_bucket.storage.id
}

output "cloudfront_public_key_id" {
  description = "The ID of the CloudFront public key used for signing."
  value       = aws_cloudfront_public_key.storage.id
}

output "ssm_private_key_parameter_name" {
  description = "The name of the SSM parameter storing the private key."
  value       = aws_ssm_parameter.cloudfront_private_key.name
}