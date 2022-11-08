#!/bin/bash
curl -H "Authorization: $1" -X PUT "$SERVER_PATH/status?status=$2" 
