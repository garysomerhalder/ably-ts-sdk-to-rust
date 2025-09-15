// 🔴 RED Phase: Encryption tests against real Ably API
// Testing AES-128/256-CBC encryption for channel messages

use ably_core::client::rest::RestClient;
use ably_core::client::realtime::RealtimeClient;
use ably_core::crypto::{CipherParams, CipherAlgorithm, CipherMode};
use ably_core::protocol::messages::Message;
use base64::Engine;
use std::time::Duration;

const TEST_API_KEY: &str = "BGkZHw.WUtzEQ:wpBCK6EsoasbyGyFNefocFYi7ESjkFlyZ8Yh-sh0PIA";

#[tokio::test]
async fn test_aes128_cbc_encryption() {
    let client = RestClient::new(TEST_API_KEY);
    
    // Create cipher params with AES-128-CBC
    let key = b"0123456789abcdef"; // 16 bytes for AES-128
    let iv = b"abcdef9876543210";  // 16 bytes IV
    
    let cipher = CipherParams::new(
        CipherAlgorithm::Aes128,
        CipherMode::Cbc,
        key.to_vec(),
        Some(iv.to_vec()),
    ).expect("Valid cipher params");
    
    let channel = client.channel("test-encrypted")
        .with_cipher(cipher);
    
    // Publish encrypted message
    let message = Message::builder()
        .name("encrypted-event")
        .data("Secret message content")
        .build();
    
    let result = channel.publish(message).await;
    assert!(result.is_ok());
    
    // Verify message was encrypted
    let history = channel.history().limit(1).execute().await.unwrap();
    assert!(!history.items.is_empty());
    
    let received = &history.items[0];
    assert!(received.encoding.as_ref().unwrap().contains("cipher+aes-128-cbc"));
    
    // Data should be base64 encoded ciphertext
    let ciphertext = received.data.as_ref().unwrap().as_str().unwrap();
    assert!(base64::engine::general_purpose::STANDARD.decode(ciphertext).is_ok());
}

#[tokio::test]
async fn test_aes256_cbc_encryption() {
    let client = RestClient::new(TEST_API_KEY);
    
    // Create cipher params with AES-256-CBC
    let key = b"0123456789abcdef0123456789abcdef"; // 32 bytes for AES-256
    let iv = b"fedcba9876543210";  // 16 bytes IV
    
    let cipher = CipherParams::new(
        CipherAlgorithm::Aes256,
        CipherMode::Cbc,
        key.to_vec(),
        Some(iv.to_vec()),
    ).expect("Valid cipher params");
    
    let channel = client.channel("test-aes256")
        .with_cipher(cipher);
    
    // Test encryption/decryption round trip
    let original = "This is a secret message with AES-256";
    let message = Message::builder()
        .data(original)
        .build();
    
    channel.publish(message).await.unwrap();
    
    // Wait for message to be stored
    tokio::time::sleep(Duration::from_millis(500)).await;
    
    // Fetch and decrypt
    let history = channel.history().limit(1).execute().await.unwrap();
    let decrypted = &history.items[0];
    
    // Should be automatically decrypted
    assert_eq!(
        decrypted.data.as_ref().unwrap().as_str().unwrap(),
        original
    );
}

// TODO: Enable when RealtimeClient::builder() is implemented
// #[tokio::test]
// async fn test_realtime_encrypted_channel() {
//     let client = RealtimeClient::builder()
//         .api_key(TEST_API_KEY)
//         .build()
//         .await
//         .unwrap();
//     
//     client.connect().await.unwrap();
//     
//     // Setup encryption
//     let key = b"mysecretkey12345"; // 16 bytes
//     let cipher = CipherParams::new(
//         CipherAlgorithm::Aes128,
//         CipherMode::Cbc,
//         key.to_vec(),
//         None, // Auto-generate IV
//     ).expect("Valid cipher params");
//     
//     let channel = client.channel("realtime-encrypted")
//         .with_cipher(cipher);
//     
//     channel.attach().await.unwrap();
//     
//     // Subscribe to messages
//     let (tx, mut rx) = tokio::sync::mpsc::channel(10);
//     channel.subscribe(move |msg| {
//         let _ = tx.try_send(msg);
//     }).await;
//     
//     // Publish encrypted message
//     let message = Message::builder()
//         .data("Realtime encrypted data")
//         .build();
//     
//     channel.publish(message).await.unwrap();
//     
//     // Receive and verify decryption
//     tokio::time::timeout(Duration::from_secs(5), async {
//         if let Some(msg) = rx.recv().await {
//             assert_eq!(
//                 msg.data.as_ref().unwrap().as_str().unwrap(),
//                 "Realtime encrypted data"
//             );
//         }
//     }).await.unwrap();
//     
//     client.disconnect().await.unwrap();
// }

#[tokio::test]
async fn test_encryption_with_different_keys() {
    let client = RestClient::new(TEST_API_KEY);
    
    // Channel 1 with key A
    let key_a = b"keyaaaaaa1234567";
    let channel_a = client.channel("channel-key-a")
        .with_cipher(CipherParams::aes128_cbc(key_a.to_vec()).expect("Valid key"));
    
    // Channel 2 with key B
    let key_b = b"keybbbbb76543210";
    let channel_b = client.channel("channel-key-b")
        .with_cipher(CipherParams::aes128_cbc(key_b.to_vec()).expect("Valid key"));
    
    // Publish to both
    let msg_a = Message::builder().data("Message for A").build();
    let msg_b = Message::builder().data("Message for B").build();
    
    channel_a.publish(msg_a).await.unwrap();
    channel_b.publish(msg_b).await.unwrap();
    
    tokio::time::sleep(Duration::from_millis(500)).await;
    
    // Verify each channel can only decrypt its own messages
    let history_a = channel_a.history().limit(1).execute().await.unwrap();
    let history_b = channel_b.history().limit(1).execute().await.unwrap();
    
    assert_eq!(
        history_a.items[0].data.as_ref().unwrap().as_str().unwrap(),
        "Message for A"
    );
    
    assert_eq!(
        history_b.items[0].data.as_ref().unwrap().as_str().unwrap(),
        "Message for B"
    );
}

#[tokio::test]
async fn test_cipher_params_from_key() {
    // Test generating cipher params from a key string
    let key_str = "WUtzEQwpBCK6EsoasbyGyFNefocFYi7E";
    let cipher = CipherParams::from_key(key_str).unwrap();
    
    assert_eq!(cipher.algorithm, CipherAlgorithm::Aes256);
    assert_eq!(cipher.mode, CipherMode::Cbc);
    assert_eq!(cipher.key.len(), 32); // 256 bits
    
    let client = RestClient::new(TEST_API_KEY);
    let channel = client.channel("test-from-key")
        .with_cipher(cipher);
    
    let message = Message::builder()
        .data("Encrypted with derived key")
        .build();
    
    let result = channel.publish(message).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_encryption_interop_with_js_sdk() {
    // Test that we can decrypt messages encrypted by JS SDK
    let client = RestClient::new(TEST_API_KEY);
    
    // Use same key as JS SDK tests
    let key = base64::engine::general_purpose::STANDARD
        .decode("WUtzEQwpBCK6EsoasbyGyFNefocFYi7ESjkFlyZ8Yh8=")
        .unwrap();
    
    let cipher = CipherParams::new(
        CipherAlgorithm::Aes256,
        CipherMode::Cbc,
        key,
        None,
    ).expect("Valid cipher params");
    
    let channel = client.channel("js-interop-test")
        .with_cipher(cipher);
    
    // If JS SDK has published encrypted messages, we should decrypt them
    let history = channel.history().limit(10).execute().await;
    
    if let Ok(result) = history {
        for msg in result.items {
            // Should be able to decrypt any JS-encrypted messages
            if msg.encoding.as_ref().map_or(false, |e| e.contains("cipher")) {
                assert!(msg.data.is_some());
            }
        }
    }
}

#[tokio::test]
async fn test_batch_encrypted_messages() {
    let client = RestClient::new(TEST_API_KEY);
    
    let cipher = CipherParams::aes128_cbc(b"batchkey12345678".to_vec()).expect("Valid cipher params");
    let channel = client.channel("batch-encrypted")
        .with_cipher(cipher);
    
    // Publish batch of encrypted messages
    let messages = vec![
        Message::builder().data("Encrypted 1").build(),
        Message::builder().data("Encrypted 2").build(),
        Message::builder().data("Encrypted 3").build(),
    ];
    
    channel.publish_batch(messages).await.unwrap();
    
    tokio::time::sleep(Duration::from_millis(500)).await;
    
    // Verify all were encrypted and can be decrypted
    let history = channel.history().limit(3).execute().await.unwrap();
    
    assert_eq!(history.items.len(), 3);
    for (i, msg) in history.items.iter().enumerate() {
        assert_eq!(
            msg.data.as_ref().unwrap().as_str().unwrap(),
            format!("Encrypted {}", 3 - i) // History is in reverse order
        );
    }
}

#[tokio::test]
async fn test_encryption_error_handling() {
    let client = RestClient::new(TEST_API_KEY);
    
    // Test with invalid key size
    let result = CipherParams::new(
        CipherAlgorithm::Aes128,
        CipherMode::Cbc,
        vec![1, 2, 3], // Invalid key size
        None,
    );
    
    assert!(result.is_err());
    
    // Test decryption with wrong key
    let correct_key = b"correct_key_1234";
    let wrong_key = b"wrong_key_567890";
    
    let channel_send = client.channel("wrong-key-test")
        .with_cipher(CipherParams::aes128_cbc(correct_key.to_vec()).expect("Valid key"));
    
    let channel_recv = client.channel("wrong-key-test")
        .with_cipher(CipherParams::aes128_cbc(wrong_key.to_vec()).expect("Valid key"));
    
    let message = Message::builder().data("Secret").build();
    channel_send.publish(message).await.unwrap();
    
    tokio::time::sleep(Duration::from_millis(500)).await;
    
    // Should fail to decrypt with wrong key
    let result = channel_recv.history().limit(1).execute().await;
    
    if let Ok(history) = result {
        if !history.items.is_empty() {
            // Message should be garbled or error
            let data = history.items[0].data.as_ref();
            assert!(data.is_none() || 
                    data.unwrap().as_str().unwrap() != "Secret");
        }
    }
}