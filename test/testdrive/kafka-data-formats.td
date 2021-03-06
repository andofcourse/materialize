# Copyright Materialize, Inc. All rights reserved.
#
# Use of this software is governed by the Business Source License
# included in the LICENSE file at the root of this repository.
#
# As of the Change Date specified in that file, in accordance with
# the Business Source License, use of this software will be governed
# by the Apache License, Version 2.0.

$ kafka-create-topic topic=input_csv
$ kafka-create-topic topic=input_proto

> CREATE MATERIALIZED SOURCE input_csv (first, second)
  FROM KAFKA BROKER '${testdrive.kafka-addr}' TOPIC 'testdrive-input_csv-${testdrive.seed}'
  FORMAT CSV WITH 2 COLUMNS;

$ kafka-ingest format=bytes topic=input_csv
1,2
2,3

> SELECT * from input_csv;
first second mz_offset
----------------------
1     2      1
2     3      2

> CREATE MATERIALIZED SOURCE input_proto
  FROM KAFKA BROKER '${testdrive.kafka-addr}' TOPIC 'testdrive-input_proto-${testdrive.seed}'
  FORMAT PROTOBUF MESSAGE '.Batch' USING SCHEMA '${testdrive.protobuf-descriptors}'

$ kafka-ingest format=protobuf topic=input_proto message=batch timestamp=1
{"id": "1", "interval_start": "2020-01-01_00:00:00", "interval_end": "2020-01-01_00:00:09", "records": []}
{"id": "2", "interval_start": "2020-01-01_00:00:10", "interval_end": "2020-01-01_00:00:19", "records": []}

# Ensure we have the offset column imported:
> SELECT * from input_proto;
id interval_start        interval_end      records mz_offset
------------------------------------------------------------
1  2020-01-01_00:00:00 2020-01-01_00:00:09 []      1
2  2020-01-01_00:00:10 2020-01-01_00:00:19 []      2
