\[  
  {  
    "PK": "USER\#Sheldon",  
    "SK": "FOLDER\#Project Docs/",  
    "ItemType": "FOLDER",  
    "FileID": "R101",  
    "OwnerID": "Sheldon",  
    "CreatedDate": 1224685700000,  
    "GSI2-PK": "USER\#Sheldon",  
    "GSI2-SK": "1224685700000\#R101"  
  },  
  {  
    "PK": "USER\#Sheldon",  
    "SK": "FILE\#Project Docs/DSCN0010.jpg",  
    "ItemType": "FILE",  
    "FileID": "R102",  
    "OwnerID": "Sheldon",  
    "CreatedDate": 1224685719000,  
    "S3Key": "Sheldon/Project Docs/DSCN0010.jpg",  
    "Size": 161713,  
    "ContentType": "image/jpeg",  
    "MediaMetadata": {  
      "type": "image",  
      "width": 640,  
      "height": 480,  
      "exif": {  
        "FocalLengthIn35mmFilm": "112",  
        "GPSSatellites": "06",  
        "ExposureMode": "auto exposure",  
        "Model": "COOLPIX P6000",  
        "PixelXDimension": "640",  
        "GPSLatitudeRef": "N",  
        "GainControl": "none",  
        "ImageDescription": "",  
        "DateTimeOriginal": "2008-10-22 16:28:39",  
        "DateTimeDigitized": "2008-10-22 16:28:39",  
        "XResolution": "72",  
        "ExposureTime": "1/75",  
        "GPSAltitudeRef": "above sea level"  
      },  
      "gps": {  
        "latitude": 43.46744833333334,  
        "longitude": 11.885126666663888,  
        "altitude": null  
      }  
    },  
    "GSI2-PK": "USER\#Sheldon",  
    "GSI2-SK": "1224685719000\#R102"  
  },  
  {  
    "PK": "USER\#Leigh",  
    "SK": "FOLDER\#Team Data/",  
    "ItemType": "FOLDER",  
    "FileID": "R201",  
    "OwnerID": "Leigh",  
    "CreatedDate": 1224686000000,  
    "GSI2-PK": "USER\#Leigh",  
    "GSI2-SK": "1224686000000\#R201"  
  },  
  {  
    "PK": "USER\#Leigh",  
    "SK": "FILE\#Team Data/Photo.jpg",  
    "ItemType": "FILE",  
    "FileID": "R202",  
    "OwnerID": "Leigh",  
    "CreatedDate": 1224686010000,  
    "S3Key": "Leigh/Team Data/Photo.jpg",  
    "Size": 2048000,  
    "ContentType": "image/jpeg",  
    "GSI2-PK": "USER\#Leigh",  
    "GSI2-SK": "1224686010000\#R202"  
  },  
  {  
    "PK": "USER\#Justin",  
    "SK": "FILE\#Private Note.txt",  
    "ItemType": "FILE",  
    "FileID": "R301",  
    "OwnerID": "Justin",  
    "CreatedDate": 1224687000000,  
    "S3Key": "Justin/Private Note.txt",  
    "Size": 1024,  
    "ContentType": "text/plain",  
    "GSI2-PK": "USER\#Justin",  
    "GSI2-SK": "1224687000000\#R301"  
  },  
  {  
    "PK": "USER\#Sheldon",  
    "SK": "GRANT\#Justin\#R101",  
    "ItemType": "SHARE\_GRANT",  
    "FileID": "R101",  
    "RecipientID": "Justin",  
    "Permissions": "READ",  
    "SharedPath": "FOLDER\#Project Docs/",  
    "GSI1-PK": "ACCESS\#Justin",  
    "GSI1-SK": "GRANT\#Sheldon\#FOLDER\#Project Docs/"  
  },  
  {  
    "PK": "USER\#Leigh",  
    "SK": "GRANT\#Sheldon\#R201",  
    "ItemType": "SHARE\_GRANT",  
    "FileID": "R201",  
    "RecipientID": "Sheldon",  
    "Permissions": "READ/WRITE",  
    "SharedPath": "FOLDER\#Team Data/",  
    "GSI1-PK": "ACCESS\#Sheldon",  
    "GSI1-SK": "GRANT\#Leigh\#FOLDER\#Team Data/"  
  },  
  {  
    "PK": "USER\#Leigh",  
    "SK": "GRANT\#Justin\#R201",  
    "ItemType": "SHARE\_GRANT",  
    "FileID": "R201",  
    "RecipientID": "Justin",  
    "Permissions": "READ/WRITE",  
    "SharedPath": "FOLDER\#Team Data/",  
    "GSI1-PK": "ACCESS\#Justin",  
    "GSI1-SK": "GRANT\#Leigh\#FOLDER\#Team Data/"  
  },  
  {  
    "PK": "USER\#Sheldon",  
    "SK": "LINK\#Sheldon\#R102",  
    "ItemType": "FEED\_LINK",  
    "FileID": "R102",  
    "OwnerID": "Sheldon",  
    "CreatedDate": 1224685719000,  
    "GSI3-PK": "FEED\#Sheldon",  
    "GSI3-SK": "1224685719000\#R102"  
  },  
  {  
    "PK": "USER\#Justin",  
    "SK": "LINK\#Sheldon\#R102",  
    "ItemType": "FEED\_LINK",  
    "FileID": "R102",  
    "OwnerID": "Sheldon",  
    "CreatedDate": 1224685719000,  
    "GSI3-PK": "FEED\#Justin",  
    "GSI3-SK": "1224685719000\#R102"  
  },  
  {  
    "PK": "USER\#Leigh",  
    "SK": "LINK\#Leigh\#R202",  
    "ItemType": "FEED\_LINK",  
    "FileID": "R202",  
    "OwnerID": "Leigh",  
    "CreatedDate": 1224686010000,  
    "GSI3-PK": "FEED\#Leigh",  
    "GSI3-SK": "1224686010000\#R202"  
  },  
  {  
    "PK": "USER\#Sheldon",  
    "SK": "LINK\#Leigh\#R202",  
    "ItemType": "FEED\_LINK",  
    "FileID": "R202",  
    "OwnerID": "Leigh",  
    "CreatedDate": 1224686010000,  
    "GSI3-PK": "FEED\#Sheldon",  
    "GSI3-SK": "1224686010000\#R202"  
  },  
  {  
    "PK": "USER\#Justin",  
    "SK": "LINK\#Leigh\#R202",  
    "ItemType": "FEED\_LINK",  
    "FileID": "R202",  
    "OwnerID": "Leigh",  
    "CreatedDate": 1224686010000,  
    "GSI3-PK": "FEED\#Justin",  
    "GSI3-SK": "1224686010000\#R202"  
  },  
  {  
    "PK": "USER\#Justin",  
    "SK": "LINK\#Justin\#R301",  
    "ItemType": "FEED\_LINK",  
    "FileID": "R301",  
    "OwnerID": "Justin",  
    "CreatedDate": 1224687000000,  
    "GSI3-PK": "FEED\#Justin",  
    "GSI3-SK": "1224687000000\#R301"  
  }  
\]  
