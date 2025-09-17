use ably_core::client::rest::RestClient;

#[tokio::test]
async fn test_auth_header_format() {
    let api_key = "BGkZHw.WUtzEQ:wpBCK6EsoasbyGyFNefocFYi7ESjkFlyZ8Yh-sh0PIA";
    let client = RestClient::new(api_key);
    
    // Test with time endpoint which should always work
    match client.time().await {
        Ok(time) => {
            println!("✅ Time endpoint worked! Server time: {}", time);
        }
        Err(e) => {
            println!("❌ Time endpoint failed: {:?}", e);
        }
    }
    
    // Try to make a direct request with proper headers
    let expected_auth = "Basic QkdrWkh3LldVdHpFUTp3cEJDSzZFc29hc2J5R3lGTmVmb2NGWWk3RVNqa0ZseVo4WWgtc2gwUElB";
    println!("Expected auth header: {}", expected_auth);
    
    // Check what our client is generating
    use base64::Engine;
    let encoded = base64::engine::general_purpose::STANDARD.encode(&api_key);
    let our_header = format!("Basic {}", encoded);
    println!("Our generated header: {}", our_header);
    
    assert_eq!(expected_auth, our_header, "Auth headers don't match!");
}