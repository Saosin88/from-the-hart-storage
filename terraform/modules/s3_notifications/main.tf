resource "aws_sqs_queue" "main" {
  name                       = "${var.name_prefix}-s3-events"
  visibility_timeout_seconds = 90
  message_retention_seconds  = 86400 # 1 days
  redrive_policy = jsonencode({
    deadLetterTargetArn = aws_sqs_queue.dlq.arn
    maxReceiveCount     = var.max_receive_count
  })
}

resource "aws_sqs_queue" "dlq" {
  name                       = "${var.name_prefix}-s3-events-dlq"
  message_retention_seconds  = 1209600 # 14 days
  visibility_timeout_seconds = 90
}

resource "aws_sqs_queue_policy" "allow_s3" {
  queue_url = aws_sqs_queue.main.id
  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Sid       = "AllowS3SendMessage"
        Effect    = "Allow"
        Principal = { Service = "s3.amazonaws.com" }
        Action    = "sqs:SendMessage"
        Resource  = aws_sqs_queue.main.arn
        Condition = {
          ArnEquals = { "aws:SourceArn" = var.bucket_arn }
        }
      }
    ]
  })
}

resource "aws_s3_bucket_notification" "to_sqs" {
  bucket = var.bucket_id

  queue {
    queue_arn = aws_sqs_queue.main.arn
    events = ["s3:ObjectCreated:Put",
      "s3:ObjectCreated:Post",
      "s3:ObjectCreated:Copy",
    "s3:ObjectCreated:CompleteMultipartUpload"]
  }

  depends_on = [aws_sqs_queue_policy.allow_s3]
}