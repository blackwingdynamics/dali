# Dali CLI Errors

## Error categories

### Input errors

The input path cannot be read, does not exist, or is not accessible. Check the
path and host permissions.

### Argument errors

A required flag is missing or an entry offset is not a valid unsigned integer.
Run the command with the required options from its command documentation.

### Cartridge construction errors

Dali cartridge rejects empty or oversized payloads, invalid entry offsets, and
output-size overflow.

### Cartridge validation errors

Dali inspect rejects invalid magic, unsupported format or target versions,
invalid ABI, invalid reserved fields, invalid payload bounds, invalid load
address, invalid entry metadata, CRC mismatches, and trailing bytes.

### Output errors

The destination cartridge cannot be written. Check the destination directory,
permissions, and available storage.

## Recovery rule

Do not copy a cartridge to hardware after a CLI error. Correct the input or
command, rerun dali inspect, and use only the successfully validated artifact.
