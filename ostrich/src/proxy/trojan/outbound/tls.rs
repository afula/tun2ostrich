use std::collections;
use std::fs;
use std::io::Write;
use std::sync::{Arc, Mutex};
// use std::net::SocketAddr;
use crate::config::TrojanOutboundSettings;
use rustls;
use rustls::{OwnedTrustAnchor, RootCertStore};
use std::str;
use webpki_roots;

/// This is an example cache for client session data.
/// It optionally dumps cached data to a file, but otherwise
/// is just in-memory.
///
/// Note that the contents of such a file are extremely sensitive.
/// Don't write this stuff to disk in production code.
struct PersistCache {
    cache: Mutex<collections::HashMap<Vec<u8>, Vec<u8>>>,
    filename: Option<String>,
}

impl PersistCache {
    /*    /// Make a new cache.  If filename is Some, load the cache
    /// from it and flush changes back to that file.
    fn new(filename: &Option<String>) -> Self {
        let cache = PersistCache {
            cache: Mutex::new(collections::HashMap::new()),
            filename: filename.clone(),
        };
        if cache.filename.is_some() {
            cache.load();
        }
        cache
    }*/

    /// If we have a filename, save the cache contents to it.
    fn save(&self) {
        use rustls::internal::msgs::base::PayloadU16;
        use rustls::internal::msgs::codec::Codec;

        if self.filename.is_none() {
            return;
        }

        let mut file =
            fs::File::create(self.filename.as_ref().unwrap()).expect("cannot open cache file");

        for (key, val) in self.cache.lock().unwrap().iter() {
            let mut item = Vec::new();
            let key_pl = PayloadU16::new(key.clone());
            let val_pl = PayloadU16::new(val.clone());
            key_pl.encode(&mut item);
            val_pl.encode(&mut item);
            file.write_all(&item).unwrap();
        }
    }
}

impl rustls::client::StoresClientSessions for PersistCache {
    /// put: insert into in-memory cache, and perhaps persist to disk.
    fn put(&self, key: Vec<u8>, value: Vec<u8>) -> bool {
        self.cache.lock().unwrap().insert(key, value);
        self.save();
        true
    }

    /// get: from in-memory cache
    fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        self.cache.lock().unwrap().get(key).cloned()
    }
}

/// Find a ciphersuite with the given name
fn find_suite(name: &str) -> Option<rustls::SupportedCipherSuite> {
    for suite in rustls::ALL_CIPHER_SUITES {
        let sname = format!("{:?}", suite.suite()).to_lowercase();

        if sname == name.to_string().to_lowercase() {
            return Some(*suite);
        }
    }

    None
}

/// Make a vector of ciphersuites named in `suites`
fn lookup_suites(suites: &String) -> Vec<rustls::SupportedCipherSuite> {
    let mut out = Vec::new();
    let suite_list: Vec<&str> = suites.split(":").collect();
    for csname in suite_list {
        let scs = find_suite(csname);
        match scs {
            Some(s) => out.push(s),
            None => panic!("cannot look up ciphersuite '{}'", csname),
        }
    }

    out
}

/// Make a vector of protocol versions named in `versions`
fn lookup_versions() -> Vec<&'static rustls::SupportedProtocolVersion> {
    let version = vec![&rustls::version::TLS13];
    version
}

#[cfg(feature = "dangerous_configuration")]
mod danger {
    use super::rustls;

    pub struct NoCertificateVerification {}

    impl rustls::client::ServerCertVerifier for NoCertificateVerification {
        fn verify_server_cert(
            &self,
            _end_entity: &rustls::Certificate,
            _intermediates: &[rustls::Certificate],
            _server_name: &rustls::ServerName,
            _scts: &mut dyn Iterator<Item = &[u8]>,
            _ocsp: &[u8],
            _now: std::time::SystemTime,
        ) -> Result<rustls::client::ServerCertVerified, rustls::Error> {
            Ok(rustls::client::ServerCertVerified::assertion())
        }
    }
}

#[cfg(feature = "dangerous_configuration")]
fn apply_dangerous_options(config: &TrojanOutboundSettings, cfg: &mut rustls::ClientConfig) {
    if args.flag_insecure {
        cfg.dangerous()
            .set_certificate_verifier(Arc::new(danger::NoCertificateVerification {}));
    }
}
//
// #[cfg(not(feature = "dangerous_configuration"))]
// fn apply_dangerous_options(args: &Args, _: &mut rustls::ClientConfig) {
//     if args.flag_insecure {
//         panic!("This build does not support --insecure.");
//     }
// }

/// Build a `ClientConfig` from our arguments
pub fn make_config(config: &TrojanOutboundSettings) -> Arc<rustls::ClientConfig> {
    let mut root_store = RootCertStore::empty();

    root_store.add_server_trust_anchors(webpki_roots::TLS_SERVER_ROOTS.0.iter().map(|ta| {
        OwnedTrustAnchor::from_subject_spki_name_constraints(
            ta.subject,
            ta.spki,
            ta.name_constraints,
        )
    }));

    let suites = if !config.suites.is_empty() {
        lookup_suites(&config.suites)
    } else {
        rustls::DEFAULT_CIPHER_SUITES.to_vec()
    };

    let versions = lookup_versions();

    let mut tls_config = rustls::ClientConfig::builder()
        .with_cipher_suites(&suites)
        .with_safe_default_kx_groups()
        .with_protocol_versions(&versions)
        .expect("inconsistent cipher-suite/versions selected")
        .with_root_certificates(root_store)
        .with_no_client_auth();

    // tls_config.key_log = Arc::new(rustls::KeyLogFile::new());

    // if args.flag_no_tickets {
    //     config.enable_tickets = false;
    // }
    //
    // if args.flag_no_sni {
    //     config.enable_sni = false;
    // }

    // config.session_storage = Arc::new(PersistCache::new(&args.flag_cache));

    // tls_config.session_storage = Arc::new(PersistCache::new(&None));
    tls_config.enable_sni = true;
    // tls_config.enable_tickets = true;
    // tls_config.enable_early_data = true;

    tls_config.alpn_protocols = config
        .alpn
        .iter()
        .map(|proto| proto.as_bytes().to_vec())
        .collect();
    // tls_config.max_fragment_size = args.flag_max_frag_size;

    // apply_dangerous_options(config, &mut tls_config);

    Arc::new(tls_config)

    // let mut root_cert_store = rustls::RootCertStore::empty();
    //
    // root_cert_store.add_server_trust_anchors(webpki_roots::TLS_SERVER_ROOTS.0.iter().map(|ta| {
    //     OwnedTrustAnchor::from_subject_spki_name_constraints(
    //         ta.subject,
    //         ta.spki,
    //         ta.name_constraints,
    //     )
    // }));
    //
    // let tls_config = rustls::ClientConfig::builder()
    //     .with_safe_defaults()
    //     .with_root_certificates(root_cert_store)
    //     .with_no_client_auth(); // i guess this was previously the default?
    // Arc::new(tls_config)
}