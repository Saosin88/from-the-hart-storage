output "file_sharing_table_name" {
  description = "DynamoDB table name for file sharing metadata"
  value       = module.file_sharing.table_name
}

output "file_sharing_table_arn" {
  description = "DynamoDB table ARN for file sharing metadata"
  value       = module.file_sharing.table_arn
}

output "file_sharing_cleanup_queue_url" {
  description = "SQS cleanup queue URL"
  value       = module.file_sharing.cleanup_queue_url
}

output "file_sharing_cleanup_queue_arn" {
  description = "SQS cleanup queue ARN"
  value       = module.file_sharing.cleanup_queue_arn
}
