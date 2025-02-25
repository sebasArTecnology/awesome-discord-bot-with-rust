use openssl::ssl::{SslConnector, SslMethod, SslVerifyMode};
use postgres::Client;
use postgres_openssl::MakeTlsConnector;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use discord::model;

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Resource {
    pub user_id: String,
    pub channel_id: String,
    pub url: String,
    pub description: String,
    pub shash: String,
    pub type_id: i32,
}

#[allow(dead_code)]
impl Resource {
    pub fn new(message: &model::Message) -> Self {
        let author_id = &message.author.id;
        let channel_id = &message.channel_id.0;
        let embeds = &message.embeds;

        let mut url: String = "".to_string();
        let mut description: String = "".to_string();

        for embed in embeds {
            let embed_title = embed.get("title").unwrap().to_string();
            let embed_description = embed.get("description").unwrap().to_string();
            url = embed.get("url").unwrap().to_string();
            url = url.to_lowercase().trim().to_string();

            description = format!(
                "{}|url: {} + {} + {}",
                description, url, embed_title, embed_description
            );
        }
        description = description.to_lowercase().trim().to_string();

        let shash = Resource::_calculate_hash(&description).to_string();

        return Self {
            user_id: author_id.to_string(),
            channel_id: channel_id.to_string(),
            url: url,
            description: description,
            type_id: 10,
            shash: shash,
        };
    }

    fn _calculate_hash<T: Hash>(t: &T) -> u64 {
        let mut s = DefaultHasher::new();
        t.hash(&mut s);
        s.finish()
    }
}

pub struct DiscordDatabase {
    db: postgres::Client,
}

#[allow(dead_code)]
impl DiscordDatabase {
    pub fn new(database_uri: String) -> Self {
        let mut builder = SslConnector::builder(SslMethod::tls()).unwrap();
        builder.set_verify(SslVerifyMode::NONE);
        let connector = MakeTlsConnector::new(builder.build());
        let db = Client::connect(&database_uri, connector).unwrap();

        return Self { db: db };
    }

    pub fn insert_resource(&mut self, resource: Resource) -> bool {
        if resource.url.is_empty() || resource.description.is_empty() {
            return false;
        }

        let query = "INSERT INTO public.resources(\
            user_id, channel_id, url, description, type_id, shash)
            VALUES ($1, $2, $3, $4, $5, $6);";

        let result = self.db.execute(
            query,
            &[
                &resource.user_id,
                &resource.channel_id,
                &resource.url,
                &resource.description,
                &resource.type_id,
                &resource.shash,
            ],
        );
        match result {
            Ok(_) => return true,
            Err(_) => return false,
        }
    }

    pub fn select_resources(&mut self, description: &str, limit: u16, page: u16) -> Vec<Resource> {
        let mut resources: Vec<Resource> = Vec::new();

        let description = format!("%{}%", description).to_string();

        let query = "SELECT * FROM resources WHERE \
            description LIKE $1 ORDER BY id DESC";

        let query = format!("{} OFFSET {} LIMIT {}", query, page * limit, limit);
        let query = query.as_str();

        let data = self.db.query(query, &[&description]).unwrap();

        for row in data {
            let url: String = row.get("url");
            let url = url.replace("\"", "");

            let description: String = row.get("description");
            let user_id: String = row.get("user_id");
            let channel_id: String = row.get("channel_id");

            let resource = Resource {
                user_id: user_id,
                channel_id: channel_id,
                url: url,
                description: description,
                ..Default::default()
            };

            resources.push(resource);
        }
        return resources;
    }

    pub fn select_random_resource(&mut self, description: &str) -> Vec<Resource> {
        let mut resources: Vec<Resource> = Vec::new();

        let description = format!("%{}%", description).to_string();

        let query = "SELECT * FROM resources WHERE \
            description LIKE $1 order by random() limit 1";
        let data = self.db.query(query, &[&description]).unwrap();

        for row in data {
            let url: String = row.get("url");
            let url = url.replace("\"", "");

            let description: String = row.get("description");

            let user_id: String = row.get("user_id");
            let channel_id: String = row.get("channel_id");

            let resource = Resource {
                user_id: user_id,
                channel_id: channel_id,
                url: url,
                description: description,
                ..Default::default()
            };

            resources.push(resource);
        }
        return resources;
    }

    pub fn _startup(mut self) {
        let instructions = vec![
            "CREATE TABLE channels
            (
              pk_channels integer NOT NULL,
              channel_id bigint NOT NULL,
              type integer NOT NULL
            );",
            "ALTER TABLE channels ADD CONSTRAINT pk_channels
            PRIMARY KEY (pk_channels);",
            "CREATE TABLE resources
            (
              resource_id integer NOT NULL,
              user_id integer NOT NULL,
              channel_id integer NOT NULL,
              url varchar(255),
              description text,
              type_id integer NOT NULL
            );",
            "ALTER TABLE resources ADD CONSTRAINT pk_resources
            PRIMARY KEY (resource_id);",
            "CREATE TABLE types
            (
              pk_types integer NOT NULL,
              type varchar(255) NOT NULL
            );",
            "ALTER TABLE types ADD CONSTRAINT pk_types
            PRIMARY KEY (pk_types);",
            "CREATE INDEX ix_channels_
            ON channels (channel_id);",
            "CREATE INDEX ix_resources_description
            ON resources (description);",
            "CREATE INDEX ix_resources_type
            ON resources (type_id);",
            "CREATE INDEX ix_resources_user
            ON resources (user_id);",
        ];
        for instruction in &instructions {
            // Create resource_type
            self.db
                .batch_execute(instruction)
                .expect("Connection error at create");
        }
    }
}
