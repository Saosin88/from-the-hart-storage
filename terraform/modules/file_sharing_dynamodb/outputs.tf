output "table_name" {
  description = "DynamoDB table name"
  value       = aws_dynamodb_table.file_metadata.name
}

output "table_arn" {
  description = "DynamoDB table ARN"
  value       = aws_dynamodb_table.file_metadata.arn
}

output "stream_arn" {
  description = "DynamoDB Streams ARN"
  value       = aws_dynamodb_table.file_metadata.stream_arn
}

output "cleanup_queue_url" {
  description = "SQS cleanup queue URL"
  value       = aws_sqs_queue.view_link_cleanup.url
}

output "cleanup_queue_arn" {
  description = "SQS cleanup queue ARN"
  value       = aws_sqs_queue.view_link_cleanup.arn
}

output "cleanup_dlq_url" {
  description = "SQS cleanup DLQ URL"
  value       = aws_sqs_queue.view_link_cleanup_dlq.url
}

output "cleanup_dlq_arn" {
  description = "SQS cleanup DLQ ARN"
  value       = aws_sqs_queue.view_link_cleanup_dlq.arn
}

output "stream_processor_function_name" {
  description = "Stream processor Lambda function name"
  value       = var.create_stream_processor ? aws_lambda_function.stream_processor[0].function_name : null
}

output "cleanup_worker_function_name" {
  description = "Cleanup worker Lambda function name"
  value       = var.create_cleanup_worker ? aws_lambda_function.cleanup_worker[0].function_name : null
}
