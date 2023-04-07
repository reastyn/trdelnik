use std::{
    io,
    sync::{Arc, Mutex, MutexGuard},
};

use tracing_subscriber::fmt::MakeWriter;

#[derive(Debug, Clone)]
pub struct MemoryWriter {
    buf: Arc<Mutex<Vec<u8>>>,
}

impl MemoryWriter {
    /// Create a new `MockWriter` that writes into the specified buffer (behind a mutex).
    pub fn new() -> Self {
        Self {
            buf: Arc::new(Mutex::new(vec![])),
        }
    }

    /// Give access to the internal buffer (behind a `MutexGuard`).
    fn buf(&self) -> io::Result<MutexGuard<Vec<u8>>> {
        // Note: The `lock` will block. This would be a problem in production code,
        // but is fine in tests.
        self.buf
            .lock()
            .map_err(|_| io::Error::from(io::ErrorKind::Other))
    }

    pub fn print(&self, curr_seq_n: usize) {
        let target = self.buf().unwrap();
        println!("Printing {} bytes:", target.len());
        println!("Checking for curr_sequence_number={curr_seq_n}");
        let resulting_log = String::from_utf8(target.to_vec()).unwrap();

        for log in resulting_log.split("\n\n") {
            let pure_string = String::from_utf8(strip_ansi_escapes::strip(log).unwrap()).unwrap();
            if pure_string.contains(format!("curr_sequence_number={curr_seq_n}").as_str()) {
                print!("{log}\n\n");
            }
        }
    }
}

impl io::Write for MemoryWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        // Lock target buffer
        let mut target = self.buf()?;

        // Write to buffer
        target.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.buf()?.flush()
    }
}

impl MakeWriter<'_> for MemoryWriter {
    type Writer = Self;

    fn make_writer(&self) -> Self::Writer {
        self.clone()
    }
}

// pub struct MakeBySequenceWriter {
//     writers: HashMap<usize, MemoryWriter>,
//     default_writer: MemoryWriter,
// }

// impl MakeBySequenceWriter {
//     pub fn new(n_seq: usize) -> Self {
//         let writers = (0..n_seq)
//             .map(|i| (i, MemoryWriter::new()))
//             .collect::<HashMap<_, _>>();
//         Self {
//             writers,
//             default_writer: MemoryWriter::new(),
//         }
//     }

//     pub fn get_writers(&self) -> HashMap<usize, MemoryWriter> {
//         self.writers.clone()
//     }
// }

// impl MakeWriter<'_> for MakeBySequenceWriter {
//     type Writer = MemoryWriter;

//     fn make_writer(&self) -> Self::Writer {
//         self.default_writer.make_writer()
//     }

//     fn make_writer_for(&self, meta: &Metadata<'_>) -> Self::Writer {
//         match meta.fields().field("curr_sequence_number") {
//             Some(i) => {
//                 let i = i.as_ref().parse::<usize>().unwrap();
//                 println!("curr_sequence_number: {}", &i);
//                 self.writers
//                     .get(&i)
//                     .expect(&format!("Writer for {i} was not found"))
//                     .clone()
//             }
//             None => self.default_writer.clone(),
//         }
//     }
// }
