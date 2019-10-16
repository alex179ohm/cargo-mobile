use ginit_core::exports::into_result::{
    command::{CommandError, CommandResult},
    IntoResult as _,
};
use openssl::{
    error::ErrorStack as OpenSslError,
    nid::Nid,
    x509::{X509NameRef, X509},
};
use std::{collections::BTreeSet, fmt, process::Command};

pub fn get_pem_list() -> CommandResult<Vec<u8>> {
    Command::new("security")
        .args(&["find-certificate", "-p", "-a", "-c", "Developer:"])
        .output()
        .into_result()
        .map(|output| output.stdout)
}

#[derive(Debug)]
pub enum Error {
    SecurityCommandFailed(CommandError),
    X509ParseFailed(OpenSslError),
    X509FieldMissing(Nid),
    FieldNotValidUtf8(OpenSslError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::SecurityCommandFailed(err) => {
                write!(f, "Failed to call `security` command: {}", err)
            }
            Error::X509ParseFailed(err) => write!(f, "Failed to parse X509 cert: {}", err),
            Error::X509FieldMissing(nid) => write!(f, "Missing X509 field: {:?}", nid),
            Error::FieldNotValidUtf8(err) => write!(f, "Field contained invalid UTF-8: {}", err),
        }
    }
}

pub fn get_x509_field(name: &X509NameRef, nid: Nid) -> Result<String, Error> {
    name.entries_by_nid(nid)
        .nth(0)
        .ok_or(Error::X509FieldMissing(nid))?
        .data()
        .as_utf8()
        .map_err(Error::FieldNotValidUtf8)
        .map(|s| s.to_string())
}

#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Team {
    pub name: String,
    pub id: String,
}

impl Team {
    pub fn from_x509(cert: X509) -> Result<Self, Error> {
        let subj = cert.subject_name();
        let name = get_x509_field(subj, Nid::ORGANIZATIONNAME)?;
        let id = get_x509_field(subj, Nid::ORGANIZATIONALUNITNAME)?;
        Ok(Self { name, id })
    }
}

pub fn find_development_teams() -> Result<Vec<Team>, Error> {
    let certs = X509::stack_from_pem(&get_pem_list().map_err(Error::SecurityCommandFailed)?)
        .map_err(Error::X509ParseFailed)?;
    let mut teams = BTreeSet::new();
    for cert in certs {
        teams.insert(Team::from_x509(cert)?);
    }
    Ok(teams.into_iter().collect())
}
