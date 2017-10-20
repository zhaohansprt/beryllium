# Beryllium

Wire bot SDK in Rust (experimental).

## Dependencies

 - [libsodium](https://github.com/jedisct1/libsodium) (required for Proteus and Cryptobox)
 - [libprotoc](https://github.com/google/protobuf) - the binary is used only for codegen (i.e., for generating the Rust source for the `messages.proto` file in root)

## Installation

### Private key and self-signed certificate

Currently, [rustls](https://github.com/ctz/rustls) only supports x509 v3 certificates with SubjectAltName extension. Make sure that your `req.cnf` looks something similar to this:

```
[req]
x509_extensions = v3_req
distinguished_name = req_distinguished_name
prompt = no

[req_distinguished_name]
C = IN
ST = TN
L = Chennai
O = Wire
OU = Bots
CN = waffles.bot

[v3_req]
basicConstraints = CA:FALSE
keyUsage = nonRepudiation, digitalSignature, keyEncipherment
subjectAltName = @alt_names

[alt_names]
DNS.1 = waffles.bot
```

 - Generate the private key, self-signed certificate and your public key.

``` bash
openssl req -nodes -newkey rsa:4096 -x509 -keyout server.key -new -out server.crt -config req.cnf -sha256 -days 7500
openssl rsa -in server.key -text > server.pem
openssl rsa -in server.key -pubout -out pubkey.pem
```

### Bot setup

 - Go to https://wire.com/b/devbot (not supported on mobile browsers, or Safari yet) - "DevBot" is used to set up your developer account and create your own bots.
 - Register yourself and create a new bot (type `help` for available commands).
 - Copy and paste the public key from `pubkey.pem`
 - Get the auth token.

### Usage

See `examples/echo-bot` for a detailed example. In context of the example, `AUTH` is the auth token, `CERT_PATH` is the path to `server.crt`, and `KEY_PATH` is for `server.pem`.

**Note:** If you're planning to launch multiple bots, then make sure that they don't share the same directory for data.
