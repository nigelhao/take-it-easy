const $ = (id) => document.getElementById(id);

const SEMITONE_MIN = -12;
const SEMITONE_MAX = 12;

let ws = null;
let semitones = 0;
let connected = false;
let serverInfo = null;
let reconnectTimer = null;

function fmtSemitones(n) {
  if (n === 0) return '0';
  return n > 0 ? `+${n}` : `${n}`;
}

function updateDisplay() {
  $('value').textContent = fmtSemitones(semitones);
  $('flat').disabled = !connected || semitones <= SEMITONE_MIN;
  $('sharp').disabled = !connected || semitones >= SEMITONE_MAX;
  $('reset').disabled = !connected || semitones === 0;
}

function setConnState(state, text) {
  const el = $('conn');
  el.classList.remove('up', 'down');
  if (state === 'up') el.classList.add('up');
  else if (state === 'down') el.classList.add('down');
  $('conn-text').textContent = text;
}

function send(obj) {
  if (ws && ws.readyState === WebSocket.OPEN) {
    ws.send(JSON.stringify(obj));
  }
}

function setSemitones(n) {
  const clamped = Math.max(SEMITONE_MIN, Math.min(SEMITONE_MAX, n));
  if (clamped === semitones) return;
  semitones = clamped;
  updateDisplay();
  send({ type: 'set', semitones });
}

function describeServer() {
  if (!serverInfo) return '';
  const algoMs = serverInfo.buffer_size > 0
    ? (1000 * serverInfo.buffer_size / serverInfo.sample_rate).toFixed(1)
    : '?';
  return `${serverInfo.sample_rate} Hz · quantum ${serverInfo.buffer_size} (${algoMs} ms)`;
}

function connect() {
  clearTimeout(reconnectTimer);
  setConnState('', 'connecting…');
  const url = `${location.protocol === 'https:' ? 'wss:' : 'ws:'}//${location.host}/ws`;
  ws = new WebSocket(url);

  ws.onopen = () => {
    connected = true;
    setConnState('up', 'connected');
    updateDisplay();
  };

  ws.onmessage = (ev) => {
    let msg;
    try { msg = JSON.parse(ev.data); } catch { return; }
    if (msg.type === 'hello') {
      serverInfo = { sample_rate: msg.sample_rate, buffer_size: msg.buffer_size };
      semitones = msg.semitones | 0;
      setConnState('up', `connected · ${describeServer()}`);
      updateDisplay();
    } else if (msg.type === 'state') {
      semitones = msg.semitones | 0;
      updateDisplay();
    }
  };

  ws.onclose = () => {
    connected = false;
    setConnState('down', 'disconnected — retrying…');
    updateDisplay();
    reconnectTimer = setTimeout(connect, 1000);
  };

  ws.onerror = () => {
    // Let onclose handle reconnect.
    try { ws.close(); } catch {}
  };
}

$('flat').addEventListener('click', () => setSemitones(semitones - 1));
$('sharp').addEventListener('click', () => setSemitones(semitones + 1));
$('reset').addEventListener('click', () => setSemitones(0));

document.addEventListener('keydown', (e) => {
  if (e.target.tagName === 'INPUT' || e.target.tagName === 'TEXTAREA') return;
  if (e.key === 'ArrowDown' || e.key === '-') { setSemitones(semitones - 1); e.preventDefault(); }
  else if (e.key === 'ArrowUp' || e.key === '+' || e.key === '=') { setSemitones(semitones + 1); e.preventDefault(); }
  else if (e.key === '0') { setSemitones(0); e.preventDefault(); }
});

updateDisplay();
connect();
