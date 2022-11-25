#!/bin/bash
curl -H "Authorization: $1" -X PUT -H "Content-Type: application/json" -d "{\"job_id\":$2,\"status\":\"$3\",\"description\":\"$4\",\"by\":\"$5\"}" "$SERVER_PATH/job"
