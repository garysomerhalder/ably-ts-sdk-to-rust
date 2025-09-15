# Ably API Credentials

## API Key

**Key ID**: `BGkZHw.WUtzEQ`  
**Full Key**: `BGkZHw.WUtzEQ:wpBCK6EsoasbyGyFNefocFYi7ESjkFlyZ8Yh-sh0PIA`

## Key Components

- **App ID**: `BGkZHw`
- **Key Name**: `WUtzEQ`
- **Key Secret**: `wpBCK6EsoasbyGyFNefocFYi7ESjkFlyZ8Yh-sh0PIA`

## Usage

This API key should be used for:
- Integration testing against Ably's sandbox/test environments
- Development and testing of the Rust SDK port
- Authenticating with Ably services during SDK development

## Security Notes

⚠️ **IMPORTANT**: This appears to be a test/sandbox key. Never commit production API keys to version control.

## Configuration Examples

### Environment Variable
```bash
export ABLY_API_KEY="BGkZHw.WUtzEQ:wpBCK6EsoasbyGyFNefocFYi7ESjkFlyZ8Yh-sh0PIA"
```

### Rust Code
```rust
let api_key = "BGkZHw.WUtzEQ:wpBCK6EsoasbyGyFNefocFYi7ESjkFlyZ8Yh-sh0PIA";
let client = ably::RestClient::new(api_key);
```

### Test Configuration
```toml
# In test configuration or .env.test
ABLY_TEST_API_KEY = "BGkZHw.WUtzEQ:wpBCK6EsoasbyGyFNefocFYi7ESjkFlyZ8Yh-sh0PIA"
```

## Related Documentation

- [Ably Authentication Documentation](https://ably.com/docs/auth)
- [Ably API Key Format](https://ably.com/docs/auth/basic)
- [Ably Sandbox Environment](https://ably.com/docs/environments)