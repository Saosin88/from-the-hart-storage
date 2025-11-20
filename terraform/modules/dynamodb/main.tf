resource "aws_dynamodb_table" "table" {
  name         = var.name
  billing_mode = "PAY_PER_REQUEST"
  hash_key     = "PK"
  range_key    = "SK"

  attribute {
    name = "PK"
    type = "S"
  }

  attribute {
    name = "SK"
    type = "S"
  }


  # attribute {
  #   name = "GSI1PK"
  #   type = "S"
  # }
  # attribute {
  #   name = "GSI1SK"
  #   type = "S"
  # }

  attribute {
    name = "GSI2PK"
    type = "S"
  }
  attribute {
    name = "GSI2SK"
    type = "S"
  }

  # global_secondary_index {
  #   name            = "grant-index"
  #   hash_key        = "GSI1PK"
  #   range_key       = "GSI1SK"
  #   projection_type = "INCLUDE"
  #   non_key_attributes = [
  #     "grant_id",
  #     "grant_type",
  #     "owner_id",
  #     "permissions",
  #     "folder_prefix",
  #     "resource_id",
  #     "file_path",
  #     "created_date",
  #   ]
  # }

  global_secondary_index {
    name            = var.gsi2_name
    hash_key        = "GSI2PK"
    range_key       = "GSI2SK"
    projection_type = "INCLUDE"
    non_key_attributes = [
      "item_type",
      "resource_id",
      "owner_id",
      "grant_id",
      "created_date",
      "name",
      "folder_prefix",
      "media_type",
      "size_bytes",
    ]
  }

  point_in_time_recovery {
    enabled = true
  }

  tags = var.tags
}
