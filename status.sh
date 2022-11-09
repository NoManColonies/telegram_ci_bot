#!/bin/bash
if [[ $3 ]]
then
  description="&description=$3"
else
  description=""
fi
if [[ $4 ]]
then
  url="&url=$4"
else
  url=""
fi
curl -H "Authorization: $1" -X PUT "$SERVER_PATH/status?status=$2$description$url"
