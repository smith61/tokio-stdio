//! Implementation of tokio_core::io::Io for stdin/stdout streams

use futures::{
    Async,
    Future
};
use futures_rb::rb;
use std::{
    io,
    thread
};
use std::io::{
    Read,
    Write
};
use tokio_core::io::{
    Io
};

/// Type that implements tokio_core::io::Io for stdin/stdout streams
pub struct Stdio {
    stdin_buffer_receiver : rb::Receiver< u8 >,
    stdout_buffer_sender  : rb::Sender< u8 >
}

fn new_would_block_error( ) -> io::Error {
    io::Error::new( io::ErrorKind::WouldBlock, "WouldBlock" )
}

fn new_pipe_broken_error( ) -> io::Error {
    io::Error::new( io::ErrorKind::BrokenPipe, "Broken pipe." )
}

impl Read for Stdio {

    fn read( &mut self, buf: &mut [ u8 ] ) -> io::Result< usize > {
        if let Async::NotReady = self.stdin_buffer_receiver.poll_read( ) {
            return Err( new_would_block_error( ) );
        }

        match self.stdin_buffer_receiver.read( buf ) {
            Ok( bytes_read ) => {
                assert!( bytes_read != 0 );

                Ok( bytes_read )
            },
            Err( _ ) => {
                Err(new_pipe_broken_error( ) )
            }
        }
    }

}

impl Write for Stdio {

    fn write( &mut self, buf: &[ u8 ] ) -> io::Result< usize > {
        if let Async::NotReady = self.stdout_buffer_sender.poll_write( ) {
            return Err( new_would_block_error( ) );
        }

        match self.stdout_buffer_sender.write( buf ) {
            Ok( bytes_written ) => {
                assert!( bytes_written != 0 );

                Ok( bytes_written )
            },
            Err( _ ) => {
                Err( new_pipe_broken_error( ) )
            }
        }
    }

    fn flush( &mut self ) -> io::Result< ( ) > {
        Ok( ( ) )
    }
}

impl Io for Stdio {

    fn poll_read( &mut self ) -> Async< ( ) > {
        self.stdin_buffer_receiver.poll_read( )
    }

    fn poll_write( &mut self ) -> Async< ( ) > {
        self.stdout_buffer_sender.poll_write( )
    }

}

impl Stdio {

    /// Create a new instance of Stdio with the given buffer sizes for stdin and stdout
    ///
    /// # Panics
    /// If either stdin_buffer_size == 0 or stdout_buffer_size == 0
    pub fn new( stdin_buffer_size : usize, stdout_buffer_size : usize ) -> Self {
        Stdio {
            stdin_buffer_receiver : Stdio::start_stdin_thread( stdin_buffer_size ),
            stdout_buffer_sender  : Stdio::start_stdout_thread( stdout_buffer_size )
        }
    }

    fn start_stdin_thread( stdin_buffer_size : usize ) -> rb::Receiver< u8 > {
        let ( mut sender, receiver ) = rb::create_buffer( stdin_buffer_size );

        thread::spawn( move | | {
            let mut stdio = io::stdin( );
            let mut buffer = [ 0u8; 1024 ];

            loop {
                let bytes_read = match stdio.read( &mut buffer ) {
                    Ok( bytes_read ) => {
                        bytes_read
                    },
                    Err( e ) => {
                        error!( "Error in stdin read thread reading from stdio: {:?}", e );
                        return;
                    }
                };

                if bytes_read == 0 {
                    trace!( "Stdin stream returned zero bytes, assuming stream closed." );
                    return;
                }

                let result = sender.write_all( &mut buffer[ .. bytes_read ] ).wait( );
                match result {
                    Ok( ret_sender ) => {
                        sender = ret_sender
                    },
                    Err( _ ) => {
                        trace!( "Stdin ring buffer receiver has been dropped, terminating stdin read thread." );
                        return;
                    }
                }
            }
        } );

        receiver
    }

    fn start_stdout_thread( stdout_buffer_size : usize ) -> rb::Sender< u8 > {
        let ( sender, mut receiver ) = rb::create_buffer( stdout_buffer_size );

        thread::spawn( move | | {
            let mut stdout = io::stdout( );
            let mut buffer = [ 0u8; 1024 ];

            loop {
                let bytes_read = match receiver.read_some( &mut buffer ).wait( ) {
                    Ok( ( bytes_read, ret_receiver ) ) => {
                        receiver = ret_receiver;

                        bytes_read
                    },
                    Err( _ ) => {
                        trace!( "Stdout ring buffer sender has been dropped, terminating stdout write thread." );
                        return;
                    }
                };

                match stdout.write_all( &buffer[ .. bytes_read ] ) {
                    Ok( _ ) => { },
                    Err( e ) => {
                        trace!( "Error in stdout write thread while writing to stdout, assuming closed: {:?}", e );
                        return;
                    }
                }
                match stdout.flush( ) {
                    Ok( _ ) => { },
                    Err( e ) => {
                        trace!( "Error in stdout write thread while flushing stdout, assuming closed: {:?}", e );
                        return;
                    }
                }
            }
        } );

        sender
    }

}