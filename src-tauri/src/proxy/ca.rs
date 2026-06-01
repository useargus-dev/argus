use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use rcgen::{CertificateParams, DistinguishedName, DnType, KeyPair, SanType};
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::RootCertStore;
use rustls_pemfile::{certs, private_key};
use tokio_rustls::rustls::ServerConfig;
use webpki_roots::TLS_SERVER_ROOTS;

use crate::db::argus_dir;
use crate::db::meta::ensure_argus_dir;
use crate::error::{AppError, AppResult};

pub fn ca_dir() -> PathBuf {
    argus_dir()
}

pub fn ca_cert_path() -> PathBuf {
    ca_dir().join("ca.pem")
}

pub fn ca_key_path() -> PathBuf {
    ca_dir().join("ca-key.pem")
}

pub fn ca_bundle_path() -> PathBuf {
    ca_dir().join("ca-bundle.pem")
}

pub fn ensure_ca_material() -> AppResult<()> {
    ensure_argus_dir()?;
    if ca_cert_path().exists() && ca_key_path().exists() {
        rebuild_bundle_if_needed()?;
        return Ok(());
    }

    let key_pair = KeyPair::generate()
        .map_err(|e| AppError::message("PROXY_CA", e.to_string()))?;
    let mut params = CertificateParams::default();
    params.distinguished_name = DistinguishedName::new();
    params
        .distinguished_name
        .push(DnType::CommonName, "Argus Dev Proxy CA");
    params.is_ca = rcgen::IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
    let cert = params
        .self_signed(&key_pair)
        .map_err(|e| AppError::message("PROXY_CA", e.to_string()))?;

    fs::write(ca_cert_path(), cert.pem())
        .map_err(|e| AppError::message("IO_ERROR", e.to_string()))?;
    fs::write(ca_key_path(), key_pair.serialize_pem())
        .map_err(|e| AppError::message("IO_ERROR", e.to_string()))?;

    rebuild_bundle()?;
    Ok(())
}

fn rebuild_bundle_if_needed() -> AppResult<()> {
    if !ca_bundle_path().exists() {
        rebuild_bundle()?;
    }
    Ok(())
}

/// SDK trust bundle: Argus CA (sufficient for trusting the local MITM proxy).
fn rebuild_bundle() -> AppResult<()> {
    let ca_pem = fs::read_to_string(ca_cert_path())
        .map_err(|e| AppError::message("IO_ERROR", e.to_string()))?;
    fs::write(ca_bundle_path(), ca_pem).map_err(|e| AppError::message("IO_ERROR", e.to_string()))?;
    Ok(())
}

pub fn issue_leaf_cert(host: &str) -> AppResult<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>)> {
    ensure_ca_material()?;
    let ca_pem = fs::read_to_string(ca_cert_path())
        .map_err(|e| AppError::message("IO_ERROR", e.to_string()))?;
    let ca_key_pem = fs::read_to_string(ca_key_path())
        .map_err(|e| AppError::message("IO_ERROR", e.to_string()))?;

    let ca_key = KeyPair::from_pem(&ca_key_pem)
        .map_err(|e| AppError::message("PROXY_CA", e.to_string()))?;
    let ca_params = CertificateParams::from_ca_cert_pem(&ca_pem)
        .map_err(|e| AppError::message("PROXY_CA", e.to_string()))?;
    let ca_cert = ca_params
        .self_signed(&ca_key)
        .map_err(|e| AppError::message("PROXY_CA", e.to_string()))?;

    let leaf_key = KeyPair::generate()
        .map_err(|e| AppError::message("PROXY_CA", e.to_string()))?;
    let mut leaf_params = CertificateParams::new(vec![host.to_string()])
        .map_err(|e| AppError::message("PROXY_CA", e.to_string()))?;
    if leaf_params.subject_alt_names.is_empty() {
        leaf_params.subject_alt_names.push(SanType::DnsName(
            host.to_string()
                .try_into()
                .map_err(|e| AppError::message("PROXY_CA", format!("{e:?}")))?,
        ));
    }
    let leaf_cert = leaf_params
        .signed_by(&leaf_key, &ca_cert, &ca_key)
        .map_err(|e| AppError::message("PROXY_CA", e.to_string()))?;

    let leaf_pem = leaf_cert.pem();
    let leaf_key_pem = leaf_key.serialize_pem();

    let leaf_certs: Vec<CertificateDer<'static>> = certs(&mut leaf_pem.as_bytes())
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| AppError::message("PROXY_CA", e.to_string()))?;
    let key = private_key(&mut leaf_key_pem.as_bytes())
        .map_err(|e| AppError::message("PROXY_CA", e.to_string()))?
        .ok_or_else(|| AppError::message("PROXY_CA", "missing leaf private key"))?;

    Ok((leaf_certs, key))
}

pub fn server_config_for_host(host: &str) -> AppResult<Arc<ServerConfig>> {
    let (certs, key) = issue_leaf_cert(host)?;
    let config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map_err(|e| AppError::message("PROXY_CA", e.to_string()))?;
    Ok(Arc::new(config))
}

pub fn upstream_root_store() -> RootCertStore {
    let mut roots = RootCertStore::empty();
    let _ = roots.extend(TLS_SERVER_ROOTS.to_vec());
    if let Ok(ca_pem) = fs::read(ca_cert_path()) {
        let mut slice = ca_pem.as_slice();
        for c in certs(&mut slice).flatten() {
            let _ = roots.add(c);
        }
    }
    roots
}
