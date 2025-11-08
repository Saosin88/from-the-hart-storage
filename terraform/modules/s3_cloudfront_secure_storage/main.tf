resource "aws_s3_bucket" "storage" {
  bucket = var.domain_name
  force_destroy = true
  tags   = var.tags
}

resource "aws_s3_bucket_versioning" "storage" {
  bucket = aws_s3_bucket.storage.id
  versioning_configuration {
    status = "Enabled"
  }
}

resource "aws_s3_bucket_server_side_encryption_configuration" "storage" {
  bucket = aws_s3_bucket.storage.id
  rule {
    apply_server_side_encryption_by_default {
      sse_algorithm = "AES256"
    }
  }
}

resource "aws_s3_bucket_public_access_block" "storage" {
  bucket                  = aws_s3_bucket.storage.id
  block_public_acls       = true
  block_public_policy     = true
  ignore_public_acls      = true
  restrict_public_buckets = true
}

resource "aws_s3_bucket_policy" "storage" {
  bucket = aws_s3_bucket.storage.id
  policy = data.aws_iam_policy_document.s3_policy.json
}

data "aws_iam_policy_document" "s3_policy" {
  statement {
    principals {
      type        = "Service"
      identifiers = ["cloudfront.amazonaws.com"]
    }

    actions = ["s3:GetObject"]
    resources = ["${aws_s3_bucket.storage.arn}/*"]

    condition {
      test     = "StringEquals"
      variable = "AWS:SourceArn"
      values   = [aws_cloudfront_distribution.storage.arn]
    }
  }
}

resource "aws_cloudfront_origin_access_control" "storage" {
  name                              = "${var.domain_name}-oac"
  description                       = "Grant cloudfront access to s3 bucket ${aws_s3_bucket.storage.id}"
  origin_access_control_origin_type = "s3"
  signing_behavior                  = "always"
  signing_protocol                  = "sigv4"
}

resource "tls_private_key" "cloudfront_private_key" {
  algorithm = "RSA"
  rsa_bits  = 2048
}

resource "aws_cloudfront_public_key" "storage" {
  comment     = "Public key for ${var.domain_name} signed URLs"
  name_prefix = "from-the-hart-tech-storage-public-key-"
  encoded_key = tls_private_key.cloudfront_private_key.public_key_pem
}

resource "aws_ssm_parameter" "cloudfront_private_key" {
  name        = var.ssm_parameter_name_for_private_key
  description = "Private key for CloudFront signed URLs for ${var.domain_name}"
  type        = "SecureString"
  value       = tls_private_key.cloudfront_private_key.private_key_pem
  tags        = var.tags
}

resource "aws_cloudfront_key_group" "storage" {
  comment = "Key group for ${var.domain_name}"
  items   = [aws_cloudfront_public_key.storage.id]
  name    = "${replace(var.domain_name, ".", "-")}-key-group"
}

resource "aws_cloudfront_cache_policy" "signed_urls" {
  name    = "${replace(var.domain_name, ".", "-")}-Signed-URL-Cache-Policy"
  comment = "Cache policy for signed URLs, ignoring signature query strings"

  parameters_in_cache_key_and_forwarded_to_origin {
    cookies_config {
      cookie_behavior = "none"
    }
    headers_config {
      header_behavior = "none"
    }
    query_strings_config {
      query_string_behavior = "allExcept"
      query_strings {
        items = ["Expires", "Signature", "Key-Pair-Id", "Policy"]
      }
    }
    enable_accept_encoding_brotli = true
    enable_accept_encoding_gzip   = true
  }

  default_ttl = 31536000 # 1 year
  max_ttl     = 31536000 # 1 year
  min_ttl     = 0
}

resource "aws_cloudfront_distribution" "storage" {
  enabled             = true
  is_ipv6_enabled     = true
  comment             = "Cloudfront for ${var.domain_name}"
  aliases             = [var.domain_name]

  origin {
    domain_name              = aws_s3_bucket.storage.bucket_regional_domain_name
    origin_id                = aws_s3_bucket.storage.bucket_regional_domain_name
    origin_access_control_id = aws_cloudfront_origin_access_control.storage.id
  }

  default_cache_behavior {
    target_origin_id       = aws_s3_bucket.storage.bucket_regional_domain_name
    viewer_protocol_policy = "redirect-to-https"
    allowed_methods        = ["GET", "HEAD", "OPTIONS"]
    cached_methods         = ["GET", "HEAD"]
    compress               = true
    cache_policy_id        = aws_cloudfront_cache_policy.signed_urls.id

    trusted_key_groups = [aws_cloudfront_key_group.storage.id]
  }

  ordered_cache_behavior {
    path_pattern           = "/public/*"
    target_origin_id       = aws_s3_bucket.storage.bucket_regional_domain_name
    viewer_protocol_policy = "redirect-to-https"
    allowed_methods        = ["GET", "HEAD", "OPTIONS"]
    cached_methods         = ["GET", "HEAD"]
    compress               = true
    cache_policy_id        = "658327ea-f89d-4fab-a63d-7e88639e58f6" # CachingOptimized policy
  }

  restrictions {
    geo_restriction {
      restriction_type = "none"
    }
  }

  viewer_certificate {
    acm_certificate_arn      = var.acm_certificate_arn
    ssl_support_method       = "sni-only"
    minimum_protocol_version = "TLSv1.2_2021"
  }

  tags = var.tags
}