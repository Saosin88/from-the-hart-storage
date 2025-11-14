*This immersive artifact is rendered using Gemini AST.*

# **File Sharing Service \- DynamoDB Technical Specification**

## **1\. Architecture Overview and Principles**

This document specifies the DynamoDB schema for the file metadata service, utilizing a **Single-Table Design** to manage file hierarchy, ownership, and complex sharing relationships with high efficiency.

### **1.1. Core Architectural Principle**

All file metadata, folder relationships, sharing permissions, and feed links are stored in a single table, partitioned by the user's ID. This provides extreme data locality for the most common access patterns (a user viewing their own files).

* **Users for Examples:** Sheldon, Leigh, and Justin.  
* **Table Name:** FileMetadata  
* **Billing Mode:** Pay-Per-Request (Serverless).

### **1.2. Schema Keys**

| Key Type | Attribute Name | Data Type | Purpose |
| :---- | :---- | :---- | :---- |
| **Partition Key (PK)** | PK | String (S) | **USER\#\<UserID\>** (Primary data locality) |
| **Sort Key (SK)** | SK | String (S) | **FILE\#\<Path\>**, **FOLDER\#\<Path\>**, **GRANT\#\<Recipient\>\#\<Path\>**, **LINK\#\<Owner\>\#\<FileID\>** |

### **1.3. Global Secondary Indexes (GSIs)**

| Index Name | Partition Key | Sort Key | Access Pattern |
| :---- | :---- | :---- | :---- |
| **GSI 1: ShareAccessIndex** | GSI1-PK (ACCESS\#\<RecipientID\>) | GSI1-SK (GRANT\#\<OwnerID\>\#\<Path\>) | Lists all items **shared with a user** ("Shared With Me" view). |
| **GSI 2: TimestampIndex** | GSI2-PK (USER\#\<OwnerID\>) | GSI2-SK (\<Timestamp\>\#\<FileID\>) | Lists an **owner's files** chronologically (Activity Stream). |
| **GSI 3: UserFeedIndex** | GSI3-PK (FEED\#\<RecipientID\>) | GSI3-SK (\<Timestamp\>\#\<FileID\>) | Generates a single, combined, chronological feed of **all accessible files** (owned and shared). |

## **2\. Detailed Data Modeling and Item Structures**

This section outlines the four core item types. For a complete JSON data set showing all these items in use, see the accompanying data\_model\_example.json file.

### **2.1. Item Type: FILE (Owned File Metadata)**

This is the canonical record for a file, stored on the **Owner's** partition. It contains the rich metadata from your example.

| Attribute | Value Format | Example (Sheldon's DSCN0010.jpg) |
| :---- | :---- | :---- |
| **PK** | USER\#\<UserID\> | USER\#Sheldon |
| **SK** | FILE\#\<FilePath\> | FILE\#Project Docs/DSCN0010.jpg |
| ItemType | FILE | FILE |
| FileID | \<UUID\> | R102 |
| CreatedDate | \<Timestamp\> | **1224685719000** |
| S3Key | \<UserID\>/\<FilePath\> | Sheldon/Project Docs/DSCN0010.jpg |
| MediaMetadata | (JSON Object) | { "type": "image", "width": 640, "height": 480, "exif": { ... } } |
| **GSI2-PK** | USER\#Sheldon |  |
| **GSI2-SK** | 1224685719000\#R102 |  |

### **2.2. Item Type: FOLDER**

Defines a hierarchical container.

| Attribute | Value Format | Example (Sheldon's Project Docs) |
| :---- | :---- | :---- |
| **PK** | USER\#\<UserID\> | USER\#Sheldon |
| **SK** | FOLDER\#\<Path\> | FOLDER\#Project Docs/ |
| ItemType | FOLDER | FOLDER |
| FileID | \<UUID\> | R101 |
| CreatedDate | \<Timestamp\> | 1224685700000 |
| **GSI2-PK** | USER\#Sheldon |  |
| **GSI2-SK** | 1224685700000\#R101 |  |

### **2.3. Item Type: SHARE\_GRANT**

Tracks permissions. Stored on the **Owner's** partition. This item is projected onto GSI 1\.

| Attribute | Value Format | Example (Sheldon → Justin) |
| :---- | :---- | :---- |
| **PK** | USER\#\<OwnerID\> | USER\#Sheldon |
| **SK** | GRANT\#\<RecipientID\>\#\<FileID\> | GRANT\#Justin\#R101 |
| ItemType | SHARE\_GRANT | SHARE\_GRANT |
| FileID | \<ResourceID\> | R101 |
| RecipientID | Justin |  |
| Permissions | READ, READ/WRITE | READ |
| SharedPath | FOLDER\#Project Docs/ |  |
| **GSI1-PK** | ACCESS\#Justin |  |
| **GSI1-SK** | GRANT\#Sheldon\#FOLDER\#Project Docs/ |  |

### **2.4. Item Type: FEED\_LINK**

Denormalized pointer for the combined feed. Stored on the **Recipient's** partition. This item is projected onto GSI 3\.

| Attribute | Value Format | Example (Justin receiving R102 from Sheldon) |
| :---- | :---- | :---- |
| **PK** | USER\#\<RecipientID\> | USER\#Justin |
| **SK** | LINK\#\<OwnerID\>\#\<FileID\> | LINK\#Sheldon\#R102 |
| ItemType | FEED\_LINK | FEED\_LINK |
| FileID | R102 |  |
| OwnerID | Sheldon |  |
| CreatedDate | 1224685719000 |  |
| **GSI3-PK** | FEED\#Justin |  |
| **GSI3-SK** | 1224685719000\#R102 |  |

## **3\. Access Patterns and Query Details (Use Cases)**

This section provides the query strategy for each use case.

### **Use Case 1: Sheldon views the contents of a folder (Project Docs/)**

Goal: List all files/folders under a specific path (e.g., Project Docs/).  
Strategy: Query the Base Table.  
// Query for files under "Project Docs/" owned by Sheldon  
const params \= {  
    TableName: 'FileMetadata',  
    KeyConditionExpression: 'PK \= :pk AND begins\_with(SK, :sk\_prefix)',  
    ExpressionAttributeValues: {  
        ':pk': 'USER\#Sheldon',  
        // Note: You would query for FILE\# and FOLDER\# prefixes to get children  
        ':sk\_prefix': 'FILE\#Project Docs/'   
    }  
};  
// Result: FILE\#Project Docs/DSCN0010.jpg

### **Use Case 2: Justin views the "Shared With Me" list**

Goal: List all items shared with Justin.  
Strategy: Query GSI 1 (ShareAccessIndex).  
// Query for all items shared with Justin  
const params \= {  
    TableName: 'FileMetadata',  
    IndexName: 'ShareAccessIndex',  
    KeyConditionExpression: 'GSI1-PK \= :pk',  
    ExpressionAttributeValues: {  
        ':pk': 'ACCESS\#Justin'  
    }  
};  
// Result: SHARE\_GRANT items from Sheldon and Leigh.

### **Use Case 3: Justin views the Global Combined Feed (Latest 50 Files)**

Goal: Show a single, chronological feed of all files accessible to Justin.  
Strategy: Query GSI 3 (UserFeedIndex) and sort descending.  
// Query the combined feed for Justin  
const params \= {  
    TableName: 'FileMetadata',  
    IndexName: 'UserFeedIndex',  
    KeyConditionExpression: 'GSI3-PK \= :pk',  
    ExpressionAttributeValues: {  
        ':pk': 'FEED\#Justin'  
    },  
    ScanIndexForward: false, // Newest first  
    Limit: 50  
};  
// Result: A sorted list of FEED\_LINK items, newest first.

### **Use Case 4: Leigh needs an activity stream of their *owned* files only**

Goal: Retrieve Leigh's own files/folders, sorted newest first.  
Strategy: Query GSI 2 (TimestampIndex) and sort descending.  
// Query Leigh's owned file activity stream  
const params \= {  
    TableName: 'FileMetadata',  
    IndexName: 'TimestampIndex',  
    KeyConditionExpression: 'GSI2-PK \= :pk',  
    ExpressionAttributeValues: {  
        ':pk': 'USER\#Leigh'  
    },  
    ScanIndexForward: false   
};  
// Result: FILE\#Team Data/Photo.jpg, FOLDER\#Team Data/

### **Use Case 5: Sheldon revokes Justin's access to Folder R101**

Goal: Remove the sharing relationship for the shared folder/file.  
Strategy: Two DeleteItem operations on the Base Table (one for the GRANT on the owner's partition, and one for the LINK on the recipient's partition). This is ideally done in a DynamoDB Transaction.  
const params \= {  
    TransactItems: \[  
        {  
            // 1\. Delete the Grant from Sheldon's partition  
            Delete: {  
                TableName: 'FileMetadata',  
                Key: {  
                    'PK': 'USER\#Sheldon',  
                    'SK': 'GRANT\#Justin\#R101'  
                }  
            }  
        },  
        {  
            // 2\. Delete the Feed Link from Justin's partition  
            // (You must find all LINK items associated with R101 for Justin)  
            Delete: {  
                TableName: 'FileMetadata',  
                Key: {  
                    'PK': 'USER\#Justin',  
                    'SK': 'LINK\#Sheldon\#R102' // Assuming R102 was the only file  
                }  
            }  
        }  
        // Note: A real implementation would need to delete links for ALL  
        // files under the shared folder R101, not just one.  
    \]  
};

## **4\. Supporting Artifacts**

* **Full Data Model:** See data\_model\_example.json for the complete dataset.  
* **Infrastructure:** See dynamodb\_table.tf for the Terraform code.