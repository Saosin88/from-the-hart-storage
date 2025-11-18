```hcl
resource "aws_dynamodb_table" "file_metadata" {
	name         = "from-the-hart-storage-dev"
	billing_mode = "PAY_PER_REQUEST"
	hash_key     = "PK"
	range_key    = "SK"

	attribute { name = "PK" type = "S" }
	attribute { name = "SK" type = "S" }

	attribute { name = "GSI1-PK" type = "S" }
	attribute { name = "GSI1-SK" type = "S" }

	attribute { name = "GSI2-PK" type = "S" }
	attribute { name = "GSI2-SK" type = "S" }

	global_secondary_index {
		name            = "grant-index"
		hash_key        = "GSI1-PK"
		range_key       = "GSI1-SK"
		projection_type = "INCLUDE"
		non_key_attributes = [
			"grant_id",
			"grant_type",
			"owner_id",
			"permissions",
			"folder_prefix",
			"resource_id",
			"file_path",
			"created_date",
		]
	}

	global_secondary_index {
		name            = "view-link-index"
		hash_key        = "GSI2-PK"
		range_key       = "GSI2-SK"
		projection_type = "INCLUDE"
		non_key_attributes = [
			"resource_id",
			"owner_id",
			"grant_id",
			"created_date",
			"file_name",
			"folder_prefix",
			"media_type",
			"size_bytes",
		]
	}

	point_in_time_recovery {
		enabled = true
	}

	tags = {
    Domain      = "tech"
    Project     = "from-the-hart-storage"
    Environment = "dev"
    Terraform   = "true"
  }
}

# Outputs for reference

output "table_name" {
	description = "DynamoDB table name"
	value       = aws_dynamodb_table.file_metadata.name
}

output "table_arn" {
	description = "DynamoDB table ARN"
	value       = aws_dynamodb_table.file_metadata.arn
}
```
