use std::net::IpAddr;
use hickory_resolver::name_server::GenericConnector;
use hickory_resolver::proto::runtime::TokioRuntimeProvider;
use hickory_resolver::Resolver;
use maxminddb::{geoip2, Mmap, Reader};
use crate::models::IpMetadata;

pub async fn lookup_ip(domain: &str, resolver: &Resolver<GenericConnector<TokioRuntimeProvider>>) -> Result<IpAddr, anyhow::Error> {
    let response = resolver.lookup_ip(domain).await?;

    let address = response.iter().next()
        .ok_or_else(|| anyhow::anyhow!("No IP addresses found for {}", domain))?;

    Ok(address)
}

pub fn lookup_ip_metadata(ip_addr: IpAddr, asn_reader: &Reader<Mmap>, country_reader: &Reader<Mmap>, city_reader: &Reader<Mmap>) -> Result<IpMetadata, anyhow::Error> {
    let asn_name = lookup_asn_organisation(ip_addr, asn_reader).ok();
    let country_iso_code = lookup_country(ip_addr, country_reader).ok();
    let city_name = lookup_city(ip_addr, city_reader).ok();
    Ok(
        IpMetadata {
            organisation: asn_name,
            country_iso_code,
            city_name
        }
    )
}

pub fn lookup_asn_organisation(ip: IpAddr, reader: &Reader<Mmap>) -> Result<String, anyhow::Error> {
    let lookup_result = reader.lookup(ip)?;

    let asn = lookup_result
        .decode::<geoip2::Asn>()?
        .ok_or_else(|| anyhow::anyhow!("IP not found in database"))?;

    let org = asn.autonomous_system_organization
        .ok_or_else(|| anyhow::anyhow!("No organization field for this record"))?;

    Ok(org.to_string())
}

pub fn lookup_country(ip: IpAddr, reader: &Reader<Mmap>) -> Result<String, anyhow::Error> {
    let lookup_result = reader.lookup(ip)?;

    let asn = lookup_result
        .decode::<geoip2::Country>()?
        .ok_or_else(|| anyhow::anyhow!("IP not found in database"))?;

    let country = asn.country.iso_code
        .ok_or_else(|| anyhow::anyhow!("No organization field for this record"))?;

    Ok(country.to_string())
}

pub fn lookup_city(ip: IpAddr, reader: &Reader<Mmap>) -> Result<String, anyhow::Error> {
    let lookup_result = reader.lookup(ip)?;

    let asn = lookup_result
        .decode::<geoip2::City>()?
        .ok_or_else(|| anyhow::anyhow!("IP not found in database"))?;

    let country = asn.city.names.english
        .ok_or_else(|| anyhow::anyhow!("No organization field for this record"))?;

    Ok(country.to_string())
}