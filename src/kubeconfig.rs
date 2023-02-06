use serde::{Deserialize, Serialize};
use std::fs;
use std::fs::File;
use std::os::unix::fs::PermissionsExt;

#[derive(Default, PartialEq, Debug, Serialize, Deserialize)]
pub struct Kubeconfig {
    #[serde(rename = "apiVersion")]
    #[serde(skip_serializing_if = "String::is_empty", default)]
    api_version: String,

    #[serde(rename = "kind")]
    #[serde(skip_serializing_if = "String::is_empty", default)]
    kind: String,

    #[serde(rename = "clusters")]
    clusters: Vec<ClusterElement>,

    #[serde(rename = "users")]
    users: Vec<UserElement>,

    #[serde(rename = "contexts")]
    contexts: Vec<ContextElement>,

    #[serde(rename = "current-context")]
    #[serde(skip_serializing_if = "String::is_empty", default)]
    current_context: String,
}

#[derive(Default, PartialEq, Debug, Serialize, Deserialize)]
pub struct ClusterElement {
    #[serde(rename = "name")]
    #[serde(skip_serializing_if = "String::is_empty", default)]
    name: String,

    #[serde(rename = "cluster")]
    cluster: ClusterCluster,
}

#[derive(Default, PartialEq, Debug, Serialize, Deserialize)]
pub struct ClusterCluster {
    #[serde(rename = "server")]
    #[serde(skip_serializing_if = "String::is_empty", default)]
    server: String,

    #[serde(rename = "certificate-authority-data")]
    #[serde(skip_serializing_if = "String::is_empty", default)]
    certificate_authority_data: String,
    // #[serde(rename = "insecure-skip-tls-verify")]
    // insecure_skip_tls_verify: bool,
}

#[derive(Default, PartialEq, Debug, Serialize, Deserialize)]
pub struct ContextElement {
    #[serde(rename = "context")]
    context: ContextContext,

    #[serde(rename = "name")]
    #[serde(skip_serializing_if = "String::is_empty", default)]
    name: String,
}

#[derive(Default, PartialEq, Debug, Serialize, Deserialize)]
pub struct ContextContext {
    #[serde(rename = "cluster")]
    cluster: String,

    #[serde(rename = "user")]
    user: String,

    #[serde(rename = "namespace")]
    #[serde(skip_serializing_if = "String::is_empty", default)]
    namespace: String,
}

#[derive(Default, PartialEq, Debug, Serialize, Deserialize)]
pub struct Preferences {}

#[derive(Default, PartialEq, Debug, Serialize, Deserialize)]
pub struct UserElement {
    #[serde(rename = "name")]
    #[serde(skip_serializing_if = "String::is_empty", default)]
    name: String,

    #[serde(rename = "user")]
    user: UserUser,
}

#[derive(Default, PartialEq, Debug, Serialize, Deserialize)]
pub struct UserUser {
    #[serde(rename = "client-certificate-data")]
    #[serde(skip_serializing_if = "String::is_empty", default)]
    client_certificate_data: String,

    #[serde(rename = "client-key-data")]
    #[serde(skip_serializing_if = "String::is_empty", default)]
    client_key_data: String,
}

impl Kubeconfig {
    // Deserialize yaml file in struc Kubeconfig
    pub fn new(path: String) -> Result<Kubeconfig, serde_yaml::Error> {
        let f = std::fs::File::open(path).expect("Could not open file.");
        serde_yaml::from_reader(f)
    }

    // Get namespace Context in Kubeconfig
    pub fn cluster_context(&self) -> String {
        if !self.contexts.is_empty() {
            return self.contexts[0].context.cluster.clone();
        };
        "".to_string()
    }

    // Get namespace Context in Kubeconfig
    pub fn namespace_context(&self) -> String {
        if !self.contexts.is_empty() {
            return self.contexts[0].context.namespace.clone();
        };
        "".to_string()
    }

    // Change namespace in Kubeconfig
    pub fn change_context(&mut self, namespace: String) {
        if !self.contexts.is_empty() {
            self.contexts[0].context.namespace = namespace;
            self.current_context = self.contexts[0].name.clone();
        };
        // ToDo change user
    }

    // Write Kubeconfig struct in yaml file
    pub fn write(&self, path: String, filename: String) {
        fs::create_dir_all(path.clone()).expect("Could not create destination dir");
        let kubefile = format!("{path}/{filename}");
        let f = File::create(kubefile.clone()).expect("File should exist");
        serde_yaml::to_writer(f, &self).unwrap();
        fs::set_permissions(kubefile, fs::Permissions::from_mode(0o600)).unwrap();
    }
}
