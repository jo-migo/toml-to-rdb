# TOML-to-RDB

CLI to convert a TOML file to an [RDB (Redis Database) file](https://redis.io/docs/latest/operate/oss_and_stack/management/persistence/). 

## Usage
Set Redis version in the environment (default = `7` if unset):
```
export REDIS_VERSION=7.2
```

Run with TOML input:
```
cat input.toml | toml-to-rdb > output.rdb
```
or with gzipped TOML input: 
```
cat input.toml.gz | toml-to-rdb -g > output.rdb
```

## Type Conversions
TOML files containing (unnested) key-value, key-table, and key-set pairs are supported.
For example, the following TOML content:
```
title = "TOML File"

[table]
k = "v"

set = [ a, b ]
```
would result in a Redis database with: 

* Key `title` having value `TOML File`
* Key `table` having [a hash value](https://redis.io/docs/latest/develop/data-types/hashes/) with key `k` and value `v`
* Key `set` having [a set value](https://redis.io/docs/latest/develop/data-types/sets/) with items `a` and `b`

