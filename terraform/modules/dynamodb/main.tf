resource "aws_dynamodb_table" "table" {
  name         = var.name
  billing_mode = "PAY_PER_REQUEST"

  attribute {
    name = "PK"
    type = "S"
  }

  attribute {
    name = "SK"
    type = "S"
  }

  key_schema = [
    { attribute = "PK", key_type = "HASH" },
    { attribute = "SK", key_type = "RANGE" },
  ]


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
  #   key_schema = [
  #     { attribute = "GSI1PK", key_type = "HASH" },
  #     { attribute = "GSI1SK", key_type = "RANGE" },
  #   ]
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
    name            = "view-link-index"
    projection_type = "INCLUDE"
    key_schema = [
      { attribute = "GSI2PK", key_type = "HASH" },
      { attribute = "GSI2SK", key_type = "RANGE" },
    ]
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
