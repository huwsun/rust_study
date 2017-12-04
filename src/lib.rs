extern crate chrono;

use chrono::prelude::*;

use std::thread;
use std::sync::mpsc;
use std::sync::Arc;
use std::sync::Mutex;

trait FnBox {
    fn call_box(self: Box<Self>,id:usize);
}

impl<F: FnOnce()> FnBox for F {
    fn call_box(self: Box<F>,id:usize)
    {

        println!("{} Worker {} receiver lock realease; job executeing...",Local::now(),id);
        (*self)()
    }
}

type Job = Box<FnBox + Send + 'static>;

enum Message {
    NewJob(Job),
    Terminate,
}


pub struct  ThreadPool{
    workers: Vec<Worker>,
    sender: mpsc::Sender<Message>,
}


pub struct Worker{
    id: usize,
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker{
     fn new(id:usize,receiver: Arc<Mutex<mpsc::Receiver<Message>>>) ->Worker{
         let thread=thread::spawn(move ||{
             loop {
                 println!("{} Worker {} begin run,request a lock",Local::now(),id);
                 let message=receiver.lock().unwrap().recv().unwrap();
                 match message {
                     Message::NewJob(job)=> {
                         println!("{} Worker {} lock receiver ,got a job; ",Local::now(),id);
                         job.call_box(id);
                         println!("{} Worker {} job finished",Local::now(),id);
                     },
                     Message::Terminate=>{
                         println!("{} Worker {} was told to terminate; ",Local::now(),id);
                         break;
                     }

                 }
             }

         });
         Worker{id,thread:Some(thread)}
     }
}


impl ThreadPool {
    /// Create a new ThreadPool.
    ///
    /// The size is the number of threads in the pool.
    ///
    /// # Panics
    ///
    /// The `new` function will panic if the size is zero.
    pub fn new(size:usize)->ThreadPool{
        assert!(size>0);

        let (sender,receiver)=mpsc::channel();
        let receiver=Arc::new(Mutex::new(receiver));

        let mut workers=Vec::with_capacity(size);

        for id in 0..size{
            workers.push(Worker::new(id,receiver.clone()));

        }

        ThreadPool{ workers ,sender}
    }

    pub fn execute<F>(&self,f:F)
    where F: FnOnce()+Send+'static
    {
        println!("{} begin request",Local::now());
        let job=Box::new(f);
        self.sender.send(Message::NewJob(job)).unwrap();
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        println!("{} Sending terminate message to all workers.",Local::now());
        for _ in &mut self.workers {
            self.sender.send(Message::Terminate).unwrap();
        }

        for worker in &mut self.workers {
            println!("{} Shutting down Worker {}",Local::now(),worker.id);
            if let Some(thread)=worker.thread.take(){
                thread.join().unwrap();
            }

            println!("{}  Worker {} finished",Local::now(),worker.id);

        }
    }
}