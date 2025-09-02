package browser_native_messaging

import (
	"bufio"
	"bytes"
	"encoding/binary"
	"encoding/json"
	"io"
	"os"
	"os/signal"
	"syscall"
	"unsafe"

	log "github.com/sirupsen/logrus"
)

type Message interface {
	RoutePath() string
	MessageID() string
}

type Response interface {
	SetInResponseTo(m Message)
}

type Listener[in Message, out Response] struct {
	log          *log.Entry
	nativeEndian binary.ByteOrder
	bufferSize   int
	handler      map[string]func(in in) (out, error)
}

func NewListener[in Message, out Response]() *Listener[in, out] {
	l := &Listener[in, out]{
		log:        log.WithField("logger", "browser-support"),
		bufferSize: 8192,
		handler:    map[string]func(in in) (out, error){},
	}
	// determine native byte order so that we can read message size correctly
	var one int16 = 1
	b := (*byte)(unsafe.Pointer(&one))
	if *b == 0 {
		l.nativeEndian = binary.BigEndian
	} else {
		l.nativeEndian = binary.LittleEndian
	}
	return l
}

func (l *Listener[in, out]) Start() {
	l.log.Debugf("Chrome native messaging host started. Native byte order: %v.", l.nativeEndian)
	l.read()
	l.log.Debug("Chrome native messaging host exited.")
}

func (l *Listener[in, out]) Handle(path string, h func(in in) (out, error)) {
	l.handler[path] = h
}

// read Creates a new buffered I/O reader and reads messages from Stdin.
func (l *Listener[in, out]) read() {
	v := bufio.NewReader(os.Stdin)
	// adjust buffer size to accommodate your json payload size limits; default is 4096
	s := bufio.NewReaderSize(v, l.bufferSize)
	l.log.Tracef("IO buffer reader created with buffer size of %v.", s.Size())

	lengthBytes := make([]byte, 4)
	lengthNum := int(0)

	go func() {
		sigChan := make(chan os.Signal, 1)
		signal.Notify(sigChan, syscall.SIGINT, syscall.SIGTERM)
		<-sigChan

		log.Info("Shutting down...")
		os.Exit(0)
	}()

	// we're going to indefinitely read the first 4 bytes in buffer, which gives us the message length.
	// if stdIn is closed we'll exit the loop and shut down host
	for b, err := s.Read(lengthBytes); b > 0 && err == nil; b, err = s.Read(lengthBytes) {
		// convert message length bytes to integer value
		lengthNum = l.readMessageLength(lengthBytes)
		l.log.Tracef("Message size in bytes: %v", lengthNum)

		// If message length exceeds size of buffer, the message will be truncated.
		// This will likely cause an error when we attempt to unmarshal message to JSON.
		if lengthNum > l.bufferSize {
			l.log.Errorf("Message size of %d exceeds buffer size of %d. Message will be truncated and is unlikely to unmarshal to JSON.", lengthNum, l.bufferSize)
		}

		// read the content of the message from buffer
		content := make([]byte, lengthNum)
		_, err := s.Read(content)
		if err != nil && err != io.EOF {
			l.log.Fatal(err)
		}

		// message has been read, now parse and process
		l.parseMessage(content)
	}

	log.Tracef("Stdin closed.")
}

// readMessageLength reads and returns the message length value in native byte order.
func (l *Listener[in, out]) readMessageLength(msg []byte) int {
	var length uint32
	buf := bytes.NewBuffer(msg)
	err := binary.Read(buf, l.nativeEndian, &length)
	if err != nil {
		l.log.WithError(err).Error("Unable to read bytes representing message length")
	}
	return int(length)
}

// parseMessage parses incoming message
func (l *Listener[in, out]) parseMessage(raw []byte) {
	msg := l.decodeMessage(raw)
	l.log.Tracef("Message received: %s", raw)

	h, ok := l.handler[msg.RoutePath()]
	if !ok {
		l.log.WithField("path", msg.RoutePath()).WithField("raw", string(raw)).Debug("Path not found")
		return
	}
	o, err := h(msg)
	if err != nil {
		l.log.WithError(err).Warning("failed to get reply")
		return
	}
	o.SetInResponseTo(msg)

	l.send(o)
}

// decodeMessage unmarshals incoming json request and returns query value.
func (l *Listener[in, out]) decodeMessage(msg []byte) in {
	var iMsg in
	err := json.Unmarshal(msg, &iMsg)
	if err != nil {
		l.log.WithError(err).Error("Unable to unmarshal json to struct")
	}
	return iMsg
}

// send sends an OutgoingMessage to os.Stdout.
func (l *Listener[in, out]) send(msg out) {
	byteMsg := l.dataToBytes(msg)
	l.writeMessageLength(byteMsg)

	var msgBuf bytes.Buffer
	_, err := msgBuf.Write(byteMsg)
	if err != nil {
		l.log.WithError(err).Error("Unable to write message length to message buffer")
	}

	_, err = msgBuf.WriteTo(os.Stdout)
	if err != nil {
		l.log.WithError(err).Error("Unable to write message buffer to Stdout")
	}
}

// dataToBytes marshals OutgoingMessage struct to slice of bytes
func (l *Listener[in, out]) dataToBytes(msg out) []byte {
	byteMsg, err := json.Marshal(msg)
	if err != nil {
		l.log.WithError(err).Error("Unable to marshal OutgoingMessage struct to slice of bytes")
	}
	return byteMsg
}

// writeMessageLength determines length of message and writes it to os.Stdout.
func (l *Listener[in, out]) writeMessageLength(msg []byte) {
	err := binary.Write(os.Stdout, l.nativeEndian, uint32(len(msg)))
	if err != nil {
		l.log.WithError(err).Error("Unable to write message length to Stdout")
	}
}
