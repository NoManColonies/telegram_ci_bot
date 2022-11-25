#!/bin/bash
curl -H "Authorization: $1" -X POST -H "Content-Type: application/json" -d "{\"job_id\":$2,\"url\":\"$3\",\"description\":\"$4\",\"by\":\"$5\",\"by_name\":\"$6\"}" "$SERVER_PATH/job"
