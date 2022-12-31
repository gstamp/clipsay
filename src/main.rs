use anyhow::Context;
use anyhow::Result;
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

struct Handler {
    english_mode: bool,
}

use anyhow::anyhow;

fn speak(clip: &str, english_mode: bool) -> Result<()> {
    let client = Client::new();
    let token = request_token(&client)?;
    let res = request_tts(clip, token, client, english_mode)?;

    let cursor = response_to_cursor(res)?;
    play_sound(cursor)?;
    Ok(())
}

fn play_sound(cursor: Cursor<Vec<u8>>) -> Result<()> {
    let (_stream, stream_handle) =
        OutputStream::try_default().expect("Unable to create output stream");
    let sink = stream_handle.play_once(cursor)?;
    sink.sleep_until_end();

    Ok(())
}

fn response_to_cursor(res: reqwest::blocking::Response) -> Result<Cursor<Vec<u8>>> {
    let sound_bytes = res.bytes()?;
    let mut c = Cursor::new(Vec::new());
    c.write_all(&sound_bytes)
        .context("Unable to read sound file data")?;
    c.seek(SeekFrom::Start(0))
        .context("Unable to seek to start of buffer")?;

    Ok(c)
}

fn request_tts(
    clip: &str,
    token: String,
    client: Client,
    english_mode: bool,
) -> Result<reqwest::blocking::Response> {
    let voice = if english_mode {
        "Microsoft Server Speech Text to Speech Voice (en-AU, NatashaNeural)"
    } else {
        "Microsoft Server Speech Text to Speech Voice (ja-JP, KeitaNeural)"
    };
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
    let res = req.send()?;

    Ok(res)
}

fn request_token(client: &Client) -> Result<String> {
    let api_key =
        env::var("AZURE_SPEECH_KEY").expect("Environment variable AZURE_SPEECH_KEY not present");
    let token = client
        .post("https://australiaeast.api.cognitive.microsoft.com/sts/v1.0/issuetoken")
        .header("Ocp-Apim-Subscription-Key", api_key)
        .header("Content-Length", 0)
        .send()
        .context("requesting TTS from azure")?
        .text()?;

    Ok(token)
}

fn clip_change_handler(english_mode: bool) -> Result<()> {
    println!("Clipboard change happened!");

    let mut ctx: ClipboardContext = ClipboardProvider::new()
        .map_err(|e| anyhow!("Unable to get clipboard context {}", e.to_string()))?;
    let text = ctx
        .get_contents()
        .map_err(|e| anyhow!("Unable to get clipboard contents {}", e.to_string()))?;
    if english_mode {
        println!("{}", text);
        speak(&text, english_mode)?
    } else {
        let re = Regex::new(r"[\u3040-\u30ff\u3400-\u4dbf\u4e00-\u9fff\uf900-\ufaff\uff66-\uff9f]")
            .expect("Unable to parse regexp");
        let is_japanese = re.is_match(&text);
        if is_japanese {
            println!("{}", text);
            speak(&text, english_mode)?
        }
    }

    Ok(())
}

impl ClipboardHandler for Handler {
    fn on_clipboard_change(&mut self) -> CallbackResult {
        if let Err(e) = clip_change_handler(self.english_mode) {
            println!("Something went wrong: {}", e)
        }

        CallbackResult::Next
    }

    fn on_clipboard_error(&mut self, error: io::Error) -> CallbackResult {
        eprintln!("Error: {}", error);

        CallbackResult::Next
    }
}

pub fn main() {
    let args: Vec<String> = env::args().collect();
    let mut english: bool = false;
    if args.len() > 1 && args[1] == "--english" {
        english = true;
        println!("Running in english mode")
    } else {
        println!("Running in japanese mode")
    }

    let _ = Master::new(Handler {
        english_mode: english,
    })
    .run();
}
