# usage guide 🐷

on first launch, the app will generate a `config.toml` file with the following structure:

```toml
[keys]
apikey = ""
secretapikey = ""

[domain]
subdomain = ""
base = ""

[ip]
address = ""
ipv6 = false
```

you can fill in your api `keys` or omit the header to read from the `PORKBUN_API_KEY` and `PORKBUN_SECRET_API_KEY` environment variables.

your `domain` information (the subdomain can be empty for base domain).

you can fill in an `address` optionally to skip the ping request that returns your ip.

enabling `ipv6` will update an AAAA address.

cool? okay. enjoy. 💜
