use std::collections::HashMap;
use std::sync::{Arc, RwLock};

#[cfg(feature = "aes256")]
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};

#[cfg(feature = "aes256")]
use rand::RngCore;

#[derive(Debug, Clone)]
pub enum EncryptionError {
    InvalidKey,
    EncryptionFailed,
    DecryptionFailed,
    KeyNotFound,
    KeyAlreadyExists,
}

impl std::fmt::Display for EncryptionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EncryptionError::InvalidKey => write!(f, "Invalid encryption key"),
            EncryptionError::EncryptionFailed => write!(f, "Encryption failed"),
            EncryptionError::DecryptionFailed => write!(f, "Decryption failed"),
            EncryptionError::KeyNotFound => write!(f, "Encryption key not found"),
            EncryptionError::KeyAlreadyExists => write!(f, "Encryption key already exists"),
        }
    }
}

impl std::error::Error for EncryptionError {}

#[cfg(feature = "aes256")]
#[derive(Clone)]
pub struct Encryptor {
    cipher: Aes256Gcm,
}

#[cfg(feature = "aes256")]
impl Encryptor {
    pub fn new(key: &[u8; 32]) -> Result<Self, EncryptionError> {
        let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| EncryptionError::InvalidKey)?;
        Ok(Self { cipher })
    }

    pub fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>, EncryptionError> {
        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        // aes-gcm 0.11（aead 0.6）把 `Nonce::from_slice` 标成了 deprecated ——
        // 它仍能编译，但会触发 `deprecated` 警告，而本仓的 clippy 门禁是
        // `-D warnings` ⇒ 必须改用 `TryFrom`。
        //
        // 长度是**静态**的：AES-GCM 的 nonce 恒为 12 字节 —— 加密路径的
        // `nonce_bytes` 是 `[u8; 12]`，解密路径已由上面的 `data.len() < 12`
        // 守卫 ⇒ 这里的转换**不可能失败**。
        let nonce = Nonce::try_from(&nonce_bytes[..]).expect("AES-GCM nonce is 12 bytes");

        let ciphertext = self
            .cipher
            .encrypt(&nonce, plaintext)
            .map_err(|_| EncryptionError::EncryptionFailed)?;

        let mut result = Vec::with_capacity(12 + ciphertext.len());
        result.extend_from_slice(&nonce_bytes);
        result.extend_from_slice(&ciphertext);
        Ok(result)
    }

    pub fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>, EncryptionError> {
        if data.len() < 12 {
            return Err(EncryptionError::DecryptionFailed);
        }

        let nonce = Nonce::try_from(&data[..12]).expect("AES-GCM nonce is 12 bytes");
        let ciphertext = &data[12..];

        self.cipher
            .decrypt(&nonce, ciphertext)
            .map_err(|_| EncryptionError::DecryptionFailed)
    }
}

#[cfg(not(feature = "aes256"))]
#[derive(Clone)]
pub struct Encryptor;

#[cfg(not(feature = "aes256"))]
impl Encryptor {
    pub fn new(_key: &[u8; 32]) -> Result<Self, EncryptionError> {
        Ok(Self)
    }

    pub fn encrypt(&self, _plaintext: &[u8]) -> Result<Vec<u8>, EncryptionError> {
        Err(EncryptionError::EncryptionFailed)
    }

    pub fn decrypt(&self, _data: &[u8]) -> Result<Vec<u8>, EncryptionError> {
        Err(EncryptionError::DecryptionFailed)
    }
}

pub struct KeyManager {
    keys: RwLock<HashMap<String, Vec<u8>>>,
    default_key_id: RwLock<Option<String>>,
}

impl KeyManager {
    pub fn new() -> Self {
        Self {
            keys: RwLock::new(HashMap::new()),
            default_key_id: RwLock::new(None),
        }
    }

    pub fn generate_key(&self, key_id: &str) -> Result<Vec<u8>, EncryptionError> {
        #[cfg(feature = "aes256")]
        {
            let mut key = vec![0u8; 32];
            rand::thread_rng().fill_bytes(&mut key);

            let mut keys = self.keys.write().unwrap();
            if keys.contains_key(key_id) {
                return Err(EncryptionError::KeyAlreadyExists);
            }

            keys.insert(key_id.to_string(), key.clone());

            let mut default = self.default_key_id.write().unwrap();
            if default.is_none() {
                *default = Some(key_id.to_string());
            }

            Ok(key)
        }

        #[cfg(not(feature = "aes256"))]
        {
            let _ = key_id;
            Err(EncryptionError::EncryptionFailed)
        }
    }

    pub fn set_default_key(&self, key_id: &str) -> Result<(), EncryptionError> {
        let keys = self.keys.read().unwrap();
        if !keys.contains_key(key_id) {
            return Err(EncryptionError::KeyNotFound);
        }
        drop(keys);

        let mut default = self.default_key_id.write().unwrap();
        *default = Some(key_id.to_string());
        Ok(())
    }

    pub fn get_key(&self, key_id: &str) -> Result<Vec<u8>, EncryptionError> {
        let keys = self.keys.read().unwrap();
        keys.get(key_id)
            .cloned()
            .ok_or(EncryptionError::KeyNotFound)
    }

    pub fn get_default_key(&self) -> Result<Vec<u8>, EncryptionError> {
        let default_id = self.default_key_id.read().unwrap();
        match default_id.as_ref() {
            Some(id) => self.get_key(id),
            None => Err(EncryptionError::KeyNotFound),
        }
    }

    pub fn delete_key(&self, key_id: &str) -> Result<(), EncryptionError> {
        let mut keys = self.keys.write().unwrap();
        if keys.remove(key_id).is_none() {
            return Err(EncryptionError::KeyNotFound);
        }

        let mut default = self.default_key_id.write().unwrap();
        if default.as_deref() == Some(key_id) {
            *default = None;
        }
        Ok(())
    }

    pub fn list_keys(&self) -> Vec<String> {
        let keys = self.keys.read().unwrap();
        keys.keys().cloned().collect()
    }

    pub fn create_encryptor(&self, key_id: Option<&str>) -> Result<Encryptor, EncryptionError> {
        let key = match key_id {
            Some(id) => self.get_key(id)?,
            None => self.get_default_key()?,
        };

        let key_array: [u8; 32] = key.try_into().map_err(|_| EncryptionError::InvalidKey)?;

        Encryptor::new(&key_array)
    }
}

impl Default for KeyManager {
    fn default() -> Self {
        Self::new()
    }
}

pub type SharedKeyManager = Arc<KeyManager>;

pub fn create_shared_key_manager() -> SharedKeyManager {
    Arc::new(KeyManager::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_manager_generate() {
        let manager = KeyManager::new();
        let key = manager.generate_key("test_key").unwrap();
        assert_eq!(key.len(), 32);
    }

    #[test]
    fn test_key_manager_default() {
        let manager = KeyManager::new();
        manager.generate_key("key1").unwrap();
        manager.generate_key("key2").unwrap();

        assert!(manager.default_key_id.read().unwrap().is_some());

        manager.set_default_key("key2").unwrap();
        assert_eq!(
            *manager.default_key_id.read().unwrap(),
            Some("key2".to_string())
        );
    }

    #[test]
    fn test_encrypt_decrypt() {
        let manager = KeyManager::new();
        manager.generate_key("default").unwrap();

        let encryptor = manager.create_encryptor(None).unwrap();
        let plaintext = b"Hello, World!";
        let ciphertext = encryptor.encrypt(plaintext).unwrap();
        let decrypted = encryptor.decrypt(&ciphertext).unwrap();

        assert_eq!(plaintext.to_vec(), decrypted);
    }

    /// **密文格式的黄金向量** —— 锁住「nonce ‖ ciphertext ‖ tag」这个布局。
    ///
    /// 为什么需要它：aes-gcm 0.10 → 0.11 是一次**主版本**升级（aead 0.5 → 0.6，
    /// 底层从 `generic-array` 换成 `hybrid-array`）。仓库此前**没有任何**跨版本的
    /// 密文兼容证据 —— 只有往返测试（同一个版本里 encrypt→decrypt），而往返测试
    /// **证明不了**「用 0.11 写的密文能被 0.10 读出来」。
    ///
    /// 这条向量是在 **aes-gcm 0.10 与 0.11 上分别实跑**得到的，两边输出
    /// **逐字节相同** ⇒ 该升级是**线上兼容**的。
    /// 以后再动 aes-gcm（或任何影响密文布局的东西），这条测试会立刻变红。
    #[test]
    fn test_ciphertext_format_golden_vector() {
        // 固定的 key / nonce / 明文 ⇒ 输出确定。
        const KEY: [u8; 32] = [0x42u8; 32];
        const NONCE: [u8; 12] = [0x24u8; 12];
        const PLAINTEXT: &[u8] = b"sqlrustgo golden vector probe";
        const EXPECTED_HEX: &str = concat!(
            "242424242424242424242424", // nonce（12 字节）
            "66e0a8339cb5b25949c845c5d156f6c8",
            "4c8d5613c2c4224dc0d6a33833b0591e",
            "bcf924e6989dc4cacc9c348250", // ciphertext ‖ tag
        );

        let cipher = Aes256Gcm::new_from_slice(&KEY).expect("32-byte key");
        let nonce = Nonce::try_from(&NONCE[..]).expect("AES-GCM nonce is 12 bytes");
        let ct = cipher.encrypt(&nonce, PLAINTEXT).expect("encrypt");

        let mut wire = NONCE.to_vec();
        wire.extend_from_slice(&ct);
        let hex: String = wire.iter().map(|b| format!("{:02x}", b)).collect();
        assert_eq!(
            hex, EXPECTED_HEX,
            "密文格式变了（aes-gcm 升级不得改变线上密文布局）"
        );

        // 再验一次：同一条密文能被解回原文（布局与解密路径自洽）。
        let back = cipher.decrypt(&nonce, ct.as_ref()).expect("decrypt");
        assert_eq!(back, PLAINTEXT);
    }

    #[test]
    fn test_key_manager_list_keys() {
        let manager = KeyManager::new();
        assert!(manager.list_keys().is_empty());
    }

    #[test]
    fn test_key_manager_generate_key() {
        let manager = KeyManager::new();
        let key = manager.generate_key("key1").unwrap();
        assert_eq!(key.len(), 32);
    }

    #[test]
    fn test_encryption_error_display() {
        let err = EncryptionError::InvalidKey;
        assert!(!format!("{}", err).is_empty());
    }
}

#[test]
fn test_encryption_error_variants() {
    assert!(!format!("{}", EncryptionError::EncryptionFailed).is_empty());
    assert!(!format!("{}", EncryptionError::DecryptionFailed).is_empty());
    assert!(!format!("{}", EncryptionError::KeyNotFound).is_empty());
    assert!(!format!("{}", EncryptionError::KeyAlreadyExists).is_empty());
}
