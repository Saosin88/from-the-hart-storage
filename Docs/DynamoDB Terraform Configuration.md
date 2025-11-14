resource "aws\_dynamodb\_table" "file\_metadata" {  
  name             \= "FileMetadata"  
  billing\_mode     \= "PAY\_PER\_REQUEST"  
  hash\_key         \= "PK"  
  range\_key        \= "SK"

  \# Base Table Attributes  
  attribute { name \= "PK" type \= "S" }  
  attribute { name \= "SK" type \= "S" }  
    
  \# GSI 1 Attributes  
  attribute { name \= "GSI1-PK" type \= "S" }  
  attribute { name \= "GSI1-SK" type \= "S" }  
    
  \# GSI 2 Attributes  
  attribute { name \= "GSI2-PK" type \= "S" }  
  attribute { name \= "GSI2-SK" type \= "S" }  
    
  \# GSI 3 Attributes  
  attribute { name \= "GSI3-PK" type \= "S" }  
  attribute { name \= "GSI3-SK" type \= "S" }

  \# GSI 1: ShareAccessIndex (Recipient's "Shared With Me" View)  
  \# Allows a recipient to find all grants they have received.  
  global\_secondary\_index {  
    name               \= "ShareAccessIndex"  
    hash\_key           \= "GSI1-PK"  
    range\_key          \= "GSI1-SK"  
    \# Project only the keys and attributes needed to display the share list  
    projection\_type    \= "INCLUDE"  
    non\_key\_attributes \= \["FileID", "OwnerID", "Permissions", "SharedPath"\]  
  }

  \# GSI 2: TimestampIndex (Owner's Chronological View)  
  \# Allows an owner to see their own activity, sorted by time.  
  global\_secondary\_index {  
    name               \= "TimestampIndex"  
    hash\_key           \= "GSI2-PK"  
    range\_key          \= "GSI2-SK"  
    \# Project attributes needed for an "activity" feed  
    projection\_type    \= "INCLUDE"  
    non\_key\_attributes \= \["FileID", "ItemType", "SK"\] \# SK has the filename  
  }

  \# GSI 3: UserFeedIndex (Global Combined Chronological Feed)  
  \# Allows any user to see a combined feed of owned \+ shared items, sorted by time.  
  global\_secondary\_index {  
    name               \= "UserFeedIndex"  
    hash\_key           \= "GSI3-PK"  
    range\_key          \= "GSI3-SK"  
    \# Project attributes needed to render the feed item  
    projection\_type    \= "INCLUDE"  
    non\_key\_attributes \= \["FileID", "OwnerID", "ItemType"\]  
  }

  tags \= {  
    Name        \= "FileServiceMetadata"  
    Environment \= "Dev"  
  }  
}  
