use clipboard_master::{CallbackResult, ClipboardHandler, Master};

use clipboard::ClipboardContext;
use clipboard::ClipboardProvider;
use regex::Regex;
use reqwest::blocking::Client;
use rodio::OutputStream;

use std::env;
use std::io;
use std::io::Seek;
use std::io::SeekFrom;
use std::io::{Cursor, Write};
struct Handler;

fn speak(clip: &str) -> () {
    let client = Client::new();
    let token = request_token(&client);
    let res = request_tts(clip, token, client);
    let cursor = response_to_cursor(res);
    play_sound(cursor);
}

fn play_sound(cursor: Cursor<Vec<u8>>) {
    let (_stream, stream_handle) =
        OutputStream::try_default().expect("Unable to create output stream");
    let sink = stream_handle.play_once(cursor).unwrap();
    sink.sleep_until_end();
}

fn response_to_cursor(res: reqwest::blocking::Response) -> Cursor<Vec<u8>> {
    let sound_bytes = res.bytes().unwrap();
    let mut c = Cursor::new(Vec::new());
    c.write_all(&sound_bytes)
        .expect("Unable to read sound file data");
    c.seek(SeekFrom::Start(0))
        .expect("Unable to seek to start of buffer");
    c
}

fn request_tts(clip: &str, token: String, client: Client) -> reqwest::blocking::Response {
    let voice = "Microsoft Server Speech Text to Speech Voice (ja-JP, KeitaNeural)";
    let body = format!("<speak version='1.0' xmlns='http://www.w3.org/2001/10/synthesis' xml:lang='en-US'><voice name='{}'>{}</voice></speak>", voice, clip);
    let mut bearer = "Bearer ".to_owned();
    bearer.push_str(&token);
    let req = client
        .post("https://australiaeast.tts.speech.microsoft.com/cognitiveservices/v1")
        .header(
            "X-Microsoft-OutputFormat",
            "audio-16khz-64kbitrate-mono-mp3",
        )
        .header("User-Agent", "australiaeast")
        .header("Content-Type", "application/ssml+xml")
        .header("Authorization", bearer)
        .body(body);
    let res = req.send().unwrap();
    res
}

fn request_token(client: &Client) -> String {
    let api_key =
        env::var("AZURE_SPEECH_KEY").expect("Environment variable AZURE_SPEECH_KEY not present");
    let token = client
        .post("https://australiaeast.api.cognitive.microsoft.com/sts/v1.0/issuetoken")
        .header("Ocp-Apim-Subscription-Key", api_key)
        .header("Content-Length", 0)
        .send()
        .unwrap()
        .text()
        .unwrap();
    token
}

impl ClipboardHandler for Handler {
    fn on_clipboard_change(&mut self) -> CallbackResult {
        println!("Clipboard change happened!");

        let mut ctx: ClipboardContext = ClipboardProvider::new().unwrap();
        let text = ctx.get_contents().unwrap();
        let re = Regex::new(r"[\u3040-\u30ff\u3400-\u4dbf\u4e00-\u9fff\uf900-\ufaff\uff66-\uff9f]")
            .unwrap();
        let is_japanese = re.is_match(&text);
        if is_japanese {
            println!("{}", text);
            speak(&text);
        }

        CallbackResult::Next
    }

    fn on_clipboard_error(&mut self, error: io::Error) -> CallbackResult {
        eprintln!("Error: {}", error);
        CallbackResult::Next
    }
}

pub fn main() {
    let _ = Master::new(Handler).run();
}
