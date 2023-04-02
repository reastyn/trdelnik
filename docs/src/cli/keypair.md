# KeyPair Command

Gets information about a keypair.

## Subcommands:

### Create

Creates a new keypair.

### Show

Shows the public key of a keypair.

### Delete

Deletes a keypair.

### Import

Imports a keypair from a seed phrase or a JSON file.

### Export

Exports a keypair to a JSON file.

### List

Lists all available keypairs.

## Example usage

To get the program ID of a key pair (key pair's public key) the `trdelnik key-pair` command can be used.
For example
```
$ trdelnik key-pair program 7
```
will print information about the key pair received from `program_keypair(7)`.