use anyhow::Context;
use futures::{TryFutureExt, TryStreamExt};
use itertools::Itertools;
use mongodb::bson::oid::ObjectId;
use mongodb::bson::{doc, Document};
use url::Url;

use super::{StreamDocument, StreamId};
use crate::internal::Optional;
use crate::storage::StorageCollection;

pub type StreamCollection = StorageCollection<StreamDocument>;

#[derive(Debug)]
pub enum StreamDocumentMatch {
    None,
    NameMatch(StreamDocument),
    UrlMatch(StreamDocument),
    MultipleMatch(Vec<StreamDocument>),
    FullMatch(StreamDocument),
}

impl StreamCollection {
    /// Adds new stream to storage.
    /// Note: pair of `name` and `url` must be unique.
    pub async fn add(self, name: &str, url: &Url) -> anyhow::Result<StreamId> {
        let id = self
            .inner()
            // `insert_one` requires a generic `Document` due to incomplete structure.
            .clone_with_type::<Document>()
            .insert_one(doc! {"name": name, "url": url.as_str()}, None)
            .await
            .map(|result| {
                result
                    .inserted_id
                    .as_object_id()
                    .expect("Insert returns ObjectId")
                    .to_hex()
            })
            .context("Adding to storage")?;
        Ok(id)
    }

    /// Looks up for a stream with `name` or `url`.
    /// Returns `StreamDocumentMatch::None` if both `name` and `url` are empty.
    pub async fn find(
        self,
        name: impl Optional<&str>,
        url: impl Optional<&Url>,
    ) -> anyhow::Result<StreamDocumentMatch> {
        let name: Option<&str> = name.into();
        let name_clause = name
            .filter(|s| !s.is_empty())
            .map(|name| doc! {"name": name});

        let url: Option<&Url> = url.into();
        let url_clause = url.map(|url| doc! {"url": url.as_str()});

        let id = |x| x;

        let clauses = [name_clause, url_clause]
            .into_iter()
            .filter_map(id)
            .collect_vec();

        if clauses.is_empty() {
            return Ok(StreamDocumentMatch::None);
        }

        let filter = match clauses.len() {
            1 => clauses.first().expect("There must be one element").clone(),
            2 => doc! {"$or": &clauses},
            _ => panic!("It must be up to 2 elements"),
        };

        let docs: Vec<StreamDocument> = self
            .inner()
            .find(filter, None)
            .and_then(|cursor| cursor.try_collect())
            .await
            .context("Filtering documents")?;

        match docs.len() {
            0 => Ok(StreamDocumentMatch::None),
            1 => {
                let doc = docs.first().cloned().expect("Must be one document");
                Ok(
                    match (
                        name.is_some() && doc.name == name.unwrap(),
                        url.is_some() && &doc.url == url.unwrap(),
                    ) {
                        (true, true) => StreamDocumentMatch::FullMatch(doc),
                        (true, false) => StreamDocumentMatch::NameMatch(doc),
                        (false, true) => StreamDocumentMatch::UrlMatch(doc),
                        (false, false) => panic!("Either name or url should match"),
                    },
                )
            }
            _ => Ok(StreamDocumentMatch::MultipleMatch(docs)),
        }
    }

    /// Gets a stream by `id`.
    pub async fn get(self, id: StreamId) -> anyhow::Result<Option<StreamDocument>> {
        let id = ObjectId::parse_str(&id)?;
        let doc = self.inner().find_one(doc! {"_id": id}, None).await?;
        Ok(doc)
    }

    /// Deletes a stream by `id`.
    /// Returns `true` if a stream has been deleted.
    /// Returns `false` if a stream does not exists.
    pub async fn delete(self, id: StreamId) -> anyhow::Result<bool> {
        let id = ObjectId::parse_str(&id)?;
        let result = self.inner().delete_one(doc! {"_id": id}, None).await?;
        Ok(result.deleted_count == 1)
    }

    pub async fn all(self) -> anyhow::Result<Vec<StreamDocument>> {
        let doc = self.inner().find(None, None).await?.try_collect().await?;
        Ok(doc)
    }
}
