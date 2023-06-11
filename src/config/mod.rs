use std::{fs::File, io::{Write, Read}, path::Path, collections::HashMap};

use serde::{Serialize, Deserialize};


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Project {
    pub slack_channel: String,
    pub github_repo: Option<String>,
    pub project_owners: Vec<String>,
    pub jira_project: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Profile {
    pub github_username: String
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Manifest {
    pub projects: HashMap<String, Project>,
    pub managers: Vec<String>,
    pub configured_project: String,
    pub profiles: HashMap<String, Profile>,
}

impl Default for Manifest {
    fn default() -> Self {
        Manifest {
            projects: HashMap::new(),
            managers: Vec::new(),
            configured_project: "amcwb/ctrl".to_string(),
            profiles: HashMap::new(),
        }
    }
}

// Utility functions for users
pub fn get_user_by_slack_id<'a>(manifest: &'a Manifest, slack_id: &str) -> Option<&'a Profile> {
    manifest.profiles.get(slack_id)
}

pub fn get_user_by_github_username<'a>(manifest: &'a Manifest, github_username: &str) -> Option<&'a Profile> {
    manifest.profiles.values().find(|profile| profile.github_username == github_username)
}

pub fn set_user_github_username(manifest: &mut Manifest, slack_id: &str, github_username: &str) {
    manifest.profiles.insert(slack_id.to_string(), Profile {
        github_username: github_username.to_string()
    });
}

pub fn get_project_by_slack_channel<'a>(manifest: &'a Manifest, slack_channel: &str) -> Option<&'a Project> {
    manifest.projects.get(slack_channel)
}

pub fn get_project_by_github_repo<'a>(manifest: &'a Manifest, github_repo: &str) -> Option<&'a Project> {
    manifest.projects.values().find(|project| project.github_repo.as_ref().unwrap_or(&"".to_string()) == github_repo)
}

pub fn get_project_by_jira_project<'a>(manifest: &'a Manifest, jira_project: &str) -> Option<&'a Project> {
    manifest.projects.values().find(|project| project.jira_project.as_ref().unwrap_or(&"".to_string()) == jira_project)
}

pub fn get_project_by_name<'a>(manifest: &'a Manifest, project_name: &str) -> Option<&'a Project> {
    manifest.projects.get(project_name)
}



pub fn write_manifest(manifest: &Manifest) {
    let mut file = File::create("manifest.toml").unwrap();

    let manifest_json = toml::to_string_pretty(&manifest).unwrap();
    file.write_all(manifest_json.as_bytes()).unwrap();
    let _ = file.sync_all();
    drop(file);

    println!("Wrote manifest.json");
    println!("{:?}", manifest);
}

pub fn read_manifest() -> Manifest {
    if !Path::new("manifest.toml").exists() {
        write_manifest(&Default::default());
    }
    
    let mut file = File::open("manifest.toml").unwrap();
    let mut contents = String::new();
    file.read_to_string(&mut contents).unwrap();

    let manifest: Manifest = toml::from_str(&contents).unwrap_or(
        Default::default()
    );

    drop(file);

    println!("Read manifest.json");
    println!("{:?}", manifest);
    manifest
}