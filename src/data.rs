use serde::Deserialize;

pub type IdBuf = str_buf::StrBuf<10>;

#[derive(Debug, Deserialize)]
pub struct Meta {
    #[allow(unused)]
    #[serde(rename = "allcount")]
    count: usize
}

#[derive(Debug, Deserialize)]
pub struct Info {
    pub title: String,
    pub ncode: IdBuf,
    pub writer: String,
    #[serde(rename = "general_all_no")]
    pub chapter_count: usize,
    #[serde(rename = "novelupdated_at")]
    pub updated_at: str_buf::StrBuf<19>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
#[allow(dead_code)]
pub enum NovelInfo {
    Meta(Meta),
    Info(Info)
}
