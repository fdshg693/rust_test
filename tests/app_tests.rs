use std::sync::mpsc::{channel, Sender, Receiver};

// Load .env before tests in this integration test binary
#[ctor::ctor]
fn _load_dotenv() { let _ = dotenvy::dotenv(); }

/// A minimal app-like struct to test state transitions without spawning the OpenAI worker.
struct TestApp {
    pub input: String,
    pub last_submitted: String,
    pub ai_answer: Option<String>,
    pub pending: bool,
    pub tx: Sender<String>,
    pub rx: Receiver<String>,
}

impl TestApp {
    fn new() -> Self {
        let (tx, rx_in) = channel::<String>();
        let (tx_out, rx) = channel::<String>();

        // Echo worker: echoes the prompt back after receiving it.
        std::thread::spawn(move || {
            while let Ok(msg) = rx_in.recv() {
                let _ = tx_out.send(format!("echo: {}", msg));
            }
        });

        Self {
            input: String::new(),
            last_submitted: String::from("(まだありません)"),
            ai_answer: None,
            pending: false,
            tx,
            rx,
        }
    }

    fn submit_prompt(&mut self) {
        if !self.input.is_empty() && !self.pending {
            self.last_submitted = self.input.clone();
            let to_send = self.input.clone();
            self.input.clear();
            self.ai_answer = None;
            self.pending = true;
            let _ = self.tx.send(to_send);
        }
    }

    fn check_ai_response(&mut self) {
        if let Ok(ans) = self.rx.try_recv() {
            self.ai_answer = Some(ans);
            self.pending = false;
        }
    }
}

#[test]
fn submit_and_receive_flow() {
    let mut app = TestApp::new();
    assert!(!app.pending);
    assert!(app.ai_answer.is_none());

    app.input = "hello".into();
    app.submit_prompt();

    // After submit, input cleared and pending true
    assert_eq!(app.last_submitted, "hello");
    assert_eq!(app.input, "");
    assert!(app.pending);
    assert!(app.ai_answer.is_none());

    // Receive echo
    // Spin a tiny wait loop to allow thread to process
    for _ in 0..50 {
        app.check_ai_response();
        if app.ai_answer.is_some() { break; }
        std::thread::sleep(std::time::Duration::from_millis(2));
    }

    assert!(!app.pending);
    assert_eq!(app.ai_answer.as_deref(), Some("echo: hello"));
}
