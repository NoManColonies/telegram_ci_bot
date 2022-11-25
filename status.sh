#!/bin/bash
if [[ $3 ]]
then
  description="&$3"
else
  description=""
fi
if [[ $4 ]]
then
  url="&$4"
else
  url=""
fi
if [[ $5 ]]
then
  by="&$5"
else
  by=""
fi
if [[ $6 ]]
then
  by_name="&$6"
else
  by_name=""
fi
curl -H "Authorization: $1" -X PUT "$SERVER_PATH/status?status=$2$description$url$by$by_name"
