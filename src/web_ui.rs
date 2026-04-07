pub fn render_web_ui(session_id: &str, _token: &str, ws_url: &str) -> String {
    let ws_url_json = serde_json::to_string(ws_url).unwrap();
    let session_short = if session_id.len() > 12 {
        &session_id[..12]
    } else {
        session_id
    };

    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover, maximum-scale=1">
<meta name="apple-mobile-web-app-capable" content="yes">
<meta name="mobile-web-app-capable" content="yes">
<meta name="theme-color" content="#1a1b26">
<title>Free CC Relay - {session_id}</title>
<style>
  * {{ margin: 0; padding: 0; box-sizing: border-box; }}
  :root {{
    --bg: #1a1b26; --surface: #24283b; --border: #3b4261;
    --text: #c0caf5; --dim: #565f89; --accent: #7aa2f7;
    --green: #9ece6a; --red: #f7768e; --yellow: #e0af68;
    --font: 'SF Mono', 'Cascadia Code', 'Fira Code', 'Consolas', monospace;
    --safe-top: env(safe-area-inset-top, 0px);
    --safe-bottom: env(safe-area-inset-bottom, 0px);
    --safe-left: env(safe-area-inset-left, 0px);
    --safe-right: env(safe-area-inset-right, 0px);
  }}
  html {{ height: 100%; }}
  body {{ background: var(--bg); color: var(--text); font-family: var(--font); height: 100%; display: flex; flex-direction: column; overflow: hidden; -webkit-text-size-adjust: 100%; }}
  #header {{ padding: 12px 16px; padding-top: calc(12px + var(--safe-top)); padding-left: calc(16px + var(--safe-left)); padding-right: calc(16px + var(--safe-right)); background: var(--surface); border-bottom: 1px solid var(--border); display: flex; align-items: center; gap: 10px; flex-shrink: 0; }}
  #header h1 {{ font-size: 14px; font-weight: 600; white-space: nowrap; }}
  #status {{ font-size: 11px; padding: 2px 8px; border-radius: 10px; white-space: nowrap; }}
  .status-connected {{ background: rgba(158,206,106,0.15); color: var(--green); }}
  .status-disconnected {{ background: rgba(247,118,142,0.15); color: var(--red); }}
  .status-waiting {{ background: rgba(224,175,104,0.15); color: var(--yellow); }}
  #session-id {{ font-size: 11px; color: var(--dim); margin-left: auto; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; min-width: 0; }}
  #messages {{ flex: 1; overflow-y: auto; padding: 12px; padding-left: calc(12px + var(--safe-left)); padding-right: calc(12px + var(--safe-right)); display: flex; flex-direction: column; gap: 8px; -webkit-overflow-scrolling: touch; overscroll-behavior: contain; }}
  .msg {{ padding: 8px 12px; border-radius: 6px; max-width: 92%; font-size: 14px; line-height: 1.6; word-wrap: break-word; overflow-wrap: break-word; }}
  .msg-user {{ background: var(--accent); color: #1a1b26; align-self: flex-end; white-space: pre-wrap; }}
  .msg-assistant {{ background: var(--surface); border: 1px solid var(--border); align-self: flex-start; max-width: 96%; }}
  .msg-system {{ background: transparent; color: var(--dim); font-size: 12px; align-self: center; font-style: italic; }}
  .msg-tool {{ background: rgba(122,162,247,0.06); border: 1px solid rgba(122,162,247,0.15); align-self: flex-start; font-size: 13px; padding: 6px 10px; max-width: 96%; }}
  .msg-label {{ font-size: 10px; color: var(--dim); margin-bottom: 2px; text-transform: uppercase; letter-spacing: 0.5px; }}
  .msg-stream {{ border-color: var(--accent); }}
  .md p {{ margin: 0.4em 0; }} .md p:first-child {{ margin-top: 0; }} .md p:last-child {{ margin-bottom: 0; }}
  .md strong {{ color: #e0e0e0; }} .md em {{ color: var(--dim); font-style: italic; }}
  .md code {{ background: rgba(0,0,0,0.35); padding: 1px 5px; border-radius: 3px; font-size: 13px; }}
  .md pre {{ background: rgba(0,0,0,0.4); padding: 8px 10px; border-radius: 4px; overflow-x: auto; margin: 0.5em 0; -webkit-overflow-scrolling: touch; }}
  .md pre code {{ background: none; padding: 0; font-size: 12px; }}
  .md ul, .md ol {{ padding-left: 1.4em; margin: 0.4em 0; }} .md li {{ margin: 0.15em 0; }}
  .md h1,.md h2,.md h3,.md h4 {{ margin: 0.6em 0 0.3em; color: #e0e0e0; }}
  .md h1 {{ font-size: 16px; }} .md h2 {{ font-size: 14px; }} .md h3 {{ font-size: 13px; }}
  .md blockquote {{ border-left: 3px solid var(--border); padding-left: 10px; color: var(--dim); margin: 0.4em 0; }}
  .md a {{ color: var(--accent); text-decoration: underline; }}
  .md hr {{ border: none; border-top: 1px solid var(--border); margin: 0.6em 0; }}
  .md table {{ border-collapse: collapse; margin: 0.5em 0; font-size: 12px; width: auto; display: block; overflow-x: auto; -webkit-overflow-scrolling: touch; }}
  .md th, .md td {{ border: 1px solid var(--border); padding: 4px 10px; text-align: left; white-space: nowrap; }}
  .md th {{ background: rgba(0,0,0,0.25); font-weight: 600; }}
  .msg-thinking {{ background: rgba(224,175,104,0.06); border: 1px solid rgba(224,175,104,0.2); align-self: flex-start; font-size: 13px; padding: 6px 10px; max-width: 96%; border-radius: 6px; }}
  .thinking-header {{ display: flex; align-items: center; gap: 6px; cursor: pointer; user-select: none; min-height: 28px; }}
  .thinking-icon {{ font-size: 12px; color: var(--yellow); }}
  .thinking-label {{ font-weight: 600; color: var(--yellow); font-size: 12px; }}
  .thinking-toggle {{ font-size: 9px; color: var(--dim); transition: transform 0.15s; display: inline-block; }}
  .thinking-toggle.open {{ transform: rotate(90deg); }}
  .thinking-content {{ display: none; margin-top: 6px; padding: 6px 8px; background: rgba(0,0,0,0.25); border-radius: 4px; font-size: 12px; color: var(--dim); max-height: 300px; overflow-y: auto; white-space: pre-wrap; -webkit-overflow-scrolling: touch; }}
  .thinking-content.open {{ display: block; }}
  .tool-header {{ display: flex; align-items: center; gap: 6px; user-select: none; flex-wrap: wrap; min-height: 32px; }}
  .tool-icon {{ font-size: 10px; color: var(--accent); }}
  .tool-name {{ font-weight: 600; color: var(--accent); font-size: 13px; }}
  .tool-summary {{ color: var(--dim); font-size: 12px; margin-left: 2px; }}
  .tool-detail {{ display: none; margin-top: 6px; padding: 6px 8px; background: rgba(0,0,0,0.3); border-radius: 4px; font-size: 12px; color: var(--dim); max-height: 200px; overflow-y: auto; white-space: pre-wrap; -webkit-overflow-scrolling: touch; }}
  .tool-detail.open {{ display: block; }}
  .tool-toggle {{ font-size: 9px; color: var(--dim); transition: transform 0.15s; display: inline-block; }}
  .tool-toggle.open {{ transform: rotate(90deg); }}
  #input-area {{ padding: 10px 12px; padding-bottom: calc(10px + var(--safe-bottom)); padding-left: calc(12px + var(--safe-left)); padding-right: calc(12px + var(--safe-right)); background: var(--surface); border-top: 1px solid var(--border); display: flex; gap: 8px; align-items: flex-end; flex-shrink: 0; }}
  #input {{ flex: 1; background: var(--bg); border: 1px solid var(--border); color: var(--text); padding: 10px 12px; border-radius: 6px; font-family: var(--font); font-size: 16px; outline: none; resize: none; min-height: 42px; max-height: 120px; line-height: 1.4; }}
  #input:focus {{ border-color: var(--accent); }}
  #input:disabled {{ opacity: 0.5; }}
  #send-btn {{ background: var(--accent); color: #1a1b26; border: none; padding: 10px 16px; border-radius: 6px; font-family: var(--font); font-size: 14px; cursor: pointer; font-weight: 600; min-height: 42px; -webkit-tap-highlight-color: transparent; }}
  #send-btn:hover {{ opacity: 0.9; }}
  #send-btn:active {{ opacity: 0.7; }}
  #send-btn:disabled {{ opacity: 0.4; cursor: not-allowed; }}
  #stop-btn {{ background: rgba(247,118,142,0.15); color: var(--red); border: 1px solid rgba(247,118,142,0.4); padding: 10px 16px; border-radius: 6px; font-family: var(--font); font-size: 14px; cursor: pointer; font-weight: 600; min-height: 42px; display: none; -webkit-tap-highlight-color: transparent; }}
  #stop-btn:hover {{ background: rgba(247,118,142,0.25); }}
  #stop-btn:active {{ opacity: 0.7; }}
  #processing-indicator {{ display: none; align-items: center; gap: 6px; font-size: 12px; color: var(--yellow); padding: 2px 8px; border-radius: 10px; background: rgba(224,175,104,0.1); white-space: nowrap; }}
  #processing-indicator.active {{ display: flex; }}
  @keyframes pulse {{ 0%,100%{{opacity:1}} 50%{{opacity:0.4}} }}
  .pulse-dot {{ width: 6px; height: 6px; border-radius: 50%; background: var(--yellow); animation: pulse 1.2s ease-in-out infinite; }}
  @media (max-width: 600px) {{
    #header h1 {{ font-size: 13px; }}
    #session-id {{ display: none; }}
    .msg {{ max-width: 95%; }}
    .msg-assistant, .msg-tool {{ max-width: 98%; }}
    .md pre {{ font-size: 11px; }}
  }}
</style>
</head>
<body>
<div id="header">
  <h1>Free CC Relay</h1>
  <span id="status" class="status-waiting">Connecting...</span>
  <span id="processing-indicator"><span class="pulse-dot"></span>Processing...</span>
  <span id="session-id">{session_short}...</span>
</div>
<div id="messages"></div>
<div id="input-area">
  <textarea id="input" rows="1" placeholder="Type a message..." autocomplete="off" disabled></textarea>
  <button id="stop-btn">Stop</button>
  <button id="send-btn" disabled>Send</button>
</div>
<script>
function escapeHtml(t){{return t.replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;')}}
function renderMarkdown(text){{
  if(!text){{var d=document.createElement('div');d.textContent='';return d;}}
  var hasCode=text.indexOf('`')!==-1;
  var hasBold=text.indexOf('**')!==-1;
  var hasHeading=/^#{{1,4}} /m.test(text);
  var hasList=/^[\-\*] /m.test(text)||/^\d+\. /m.test(text);
  var hasTable=/^\|.+\|$/m.test(text);
  var hasLink=/\[.+\]\(.+\)/.test(text);
  var hasBlockquote=/^> /m.test(text);
  var hasHr=/^---$/m.test(text);
  var hasMd=hasCode||hasBold||hasHeading||hasList||hasTable||hasLink||hasBlockquote||hasHr;
  if(!hasMd){{var d=document.createElement('div');d.textContent=text;d.style.whiteSpace='pre-wrap';return d;}}
  var html='';var lines=text.split('\n');var i=0;
  var inCodeBlock=false;var codeContent='';var inTable=false;var tableRows=[];var inList=false;var listType='';
  function flushList(){{if(!inList)return;html+='</'+listType+'>';inList=false;}}
  function flushTable(){{if(!inTable)return;html+='<table>';for(var r=0;r<tableRows.length;r++){{var cells=tableRows[r].replace(/^\|/,'').replace(/\|$/,'').split('|');if(cells.every(function(c){{return/^[\s:\-]+$/.test(c);}}))continue;var tag=r===0?'th':'td';html+='<tr>';for(var c=0;c<cells.length;c++){{html+='<'+tag+'>'+inlineFormat(cells[c].trim())+'</'+tag+'>';}}html+='</tr>';}}html+='</table>';tableRows=[];inTable=false;}}
  function inlineFormat(s){{s=escapeHtml(s);s=s.replace(/`([^`]+)`/g,'<code>$1</code>');s=s.replace(/\*\*([^*]+)\*\*/g,'<strong>$1</strong>');s=s.replace(/(?<!\*)\*([^*]+)\*(?!\*)/g,'<em>$1</em>');s=s.replace(/\[([^\]]+)\]\(([^)]+)\)/g,'<a href="$2" target="_blank" rel="noopener">$1</a>');return s;}}
  while(i<lines.length){{var line=lines[i];
    if(line.trimStart().startsWith('```')){{if(inCodeBlock){{html+='<pre><code>'+escapeHtml(codeContent.trimEnd())+'</code></pre>';codeContent='';inCodeBlock=false;i++;continue;}}else{{flushList();flushTable();inCodeBlock=true;i++;continue;}}}}
    if(inCodeBlock){{codeContent+=line+'\n';i++;continue;}}
    if(/^\|.+\|$/.test(line.trim())){{flushList();if(!inTable)inTable=true;tableRows.push(line.trim());i++;continue;}}else{{flushTable();}}
    var headingMatch=line.match(/^(#{{1,4}}) (.+)$/);if(headingMatch){{flushList();var level=headingMatch[1].length;html+='<h'+level+'>'+inlineFormat(headingMatch[2])+'</h'+level+'>';i++;continue;}}
    if(/^---$/.test(line.trim())){{flushList();html+='<hr>';i++;continue;}}
    if(line.startsWith('> ')){{flushList();html+='<blockquote>'+inlineFormat(line.slice(2))+'</blockquote>';i++;continue;}}
    var ulMatch=line.match(/^[\-\*] (.+)$/);if(ulMatch){{flushTable();if(!inList||listType!=='ul'){{flushList();html+='<ul>';inList=true;listType='ul';}}html+='<li>'+inlineFormat(ulMatch[1])+'</li>';i++;continue;}}
    var olMatch=line.match(/^\d+\. (.+)$/);if(olMatch){{flushTable();if(!inList||listType!=='ol'){{flushList();html+='<ol>';inList=true;listType='ol';}}html+='<li>'+inlineFormat(olMatch[1])+'</li>';i++;continue;}}
    flushList();if(line.trim()===''){{i++;continue;}}
    html+='<p>'+inlineFormat(line)+'</p>';i++;
  }}
  if(inCodeBlock)html+='<pre><code>'+escapeHtml(codeContent.trimEnd())+'</code></pre>';
  flushList();flushTable();
  var div=document.createElement('div');div.className='md';div.innerHTML=html;return div;
}}

var messagesEl=document.getElementById('messages');
var inputEl=document.getElementById('input');
var sendBtn=document.getElementById('send-btn');
var stopBtn=document.getElementById('stop-btn');
var statusEl=document.getElementById('status');
var processingEl=document.getElementById('processing-indicator');
var ws,currentStreamId=null,currentStreamEl=null,reconnectTimer=null,isProcessing=false;

function setProcessing(active){{
  isProcessing=active;
  processingEl.classList.toggle('active',active);
  stopBtn.style.display=active?'block':'none';
}}

function setStatus(text,cls){{statusEl.textContent=text;statusEl.className=cls;}}

function addMessage(role,content,id){{
  var div=document.createElement('div');div.className='msg msg-'+role;if(id)div.dataset.id=id;
  if(role!=='system'&&role!=='tool'){{var label=document.createElement('div');label.className='msg-label';label.textContent=role==='user'?'You':role==='assistant'?'Claude':role;div.appendChild(label);}}
  if(role==='assistant'){{div.appendChild(renderMarkdown(content||''));}}else{{var t=document.createElement('div');t.textContent=content;if(role==='user')t.style.whiteSpace='pre-wrap';div.appendChild(t);}}
  messagesEl.appendChild(div);messagesEl.scrollTop=messagesEl.scrollHeight;return div;
}}

function addThinkingMessage(content){{
  var div=document.createElement('div');div.className='msg msg-thinking';
  var header=document.createElement('div');header.className='thinking-header';
  var toggle=document.createElement('span');toggle.className='thinking-toggle';toggle.textContent='\u25B6';
  var icon=document.createElement('span');icon.className='thinking-icon';icon.textContent='\u2234';
  var label=document.createElement('span');label.className='thinking-label';label.textContent='Thinking';
  header.appendChild(toggle);header.appendChild(icon);header.appendChild(label);div.appendChild(header);
  if(content){{var body=document.createElement('div');body.className='thinking-content';body.textContent=content;div.appendChild(body);header.addEventListener('click',function(){{body.classList.toggle('open');toggle.classList.toggle('open');}});}}
  messagesEl.appendChild(div);messagesEl.scrollTop=messagesEl.scrollHeight;return div;
}}

function addToolMessage(name,summary,detail){{
  var div=document.createElement('div');div.className='msg msg-tool';
  var header=document.createElement('div');header.className='tool-header';
  var toggle=document.createElement('span');toggle.className='tool-toggle';toggle.textContent='\u25B6';
  var icon=document.createElement('span');icon.className='tool-icon';icon.textContent='\u2699';
  var nameEl=document.createElement('span');nameEl.className='tool-name';nameEl.textContent=name||'Tool';
  header.appendChild(toggle);header.appendChild(icon);header.appendChild(nameEl);div.appendChild(header);
  if(summary){{var s=document.createElement('div');s.className='tool-summary';s.textContent=summary;div.appendChild(s);}}
  if(detail){{var de=document.createElement('div');de.className='tool-detail';de.textContent=detail;div.appendChild(de);header.addEventListener('click',function(){{de.classList.toggle('open');toggle.classList.toggle('open');}});}}
  messagesEl.appendChild(div);messagesEl.scrollTop=messagesEl.scrollHeight;return div;
}}

function connect(){{
  ws=new WebSocket({ws_url_json});
  ws.onopen=function(){{setStatus('Connected','status-connected');inputEl.disabled=false;sendBtn.disabled=false;inputEl.focus();}};
  ws.onclose=function(){{setStatus('Disconnected','status-disconnected');inputEl.disabled=true;sendBtn.disabled=true;setProcessing(false);reconnectTimer=setTimeout(connect,3000);}};
  ws.onerror=function(){{}};
  ws.onmessage=function(event){{
    var msg;try{{msg=JSON.parse(event.data);}}catch(e){{return;}}
    switch(msg.type){{
      case 'history':messagesEl.innerHTML='';for(var i=0;i<msg.messages.length;i++){{var m=msg.messages[i];if(m.type==='tool_use')addToolMessage(m.name,m.content,m.detail);else if(m.type==='tool_result')addToolMessage('Result',m.content,m.detail);else if(m.type==='thinking')addThinkingMessage(m.content);else addMessage(m.role||m.type,m.content||'',m.id);}}break;
      case 'session_info':setStatus(msg.cliConnected?'CLI Connected':'Waiting for CLI',msg.cliConnected?'status-connected':'status-waiting');break;
      case 'message':addMessage(msg.role,msg.content,msg.id);break;
      case 'stream_start':currentStreamId=msg.id;currentStreamEl=addMessage('assistant','',msg.id);currentStreamEl.classList.add('msg-stream');break;
      case 'stream_delta':if(currentStreamEl&&msg.id===currentStreamId){{var te=currentStreamEl.querySelector('div:last-child');if(te){{if(!te._raw)te._raw='';te._raw+=msg.content;te.textContent=te._raw;}}messagesEl.scrollTop=messagesEl.scrollHeight;}}break;
      case 'stream_end':if(currentStreamEl){{var te2=currentStreamEl.querySelector('div:last-child');if(te2&&te2._raw){{te2.replaceWith(renderMarkdown(te2._raw));}}currentStreamEl.classList.remove('msg-stream');currentStreamEl=null;currentStreamId=null;messagesEl.scrollTop=messagesEl.scrollHeight;}}break;
      case 'tool_use':addToolMessage(msg.name,msg.content,msg.detail);break;
      case 'tool_result':addToolMessage('Result',msg.content,msg.detail);break;
      case 'thinking':addThinkingMessage(msg.content);break;
      case 'status':setProcessing(!!msg.processing);break;
      case 'system':addMessage('system',msg.content);break;
    }}
  }};
}}

function send(){{
  var text=inputEl.value.trim();if(!text||!ws||ws.readyState!==WebSocket.OPEN)return;
  ws.send(JSON.stringify({{type:'message',role:'user',content:text,id:'web_'+Date.now(),timestamp:Date.now()}}));
  addMessage('user',text);inputEl.value='';inputEl.focus();
  setProcessing(true);
}}

function stop(){{
  if(!ws||ws.readyState!==WebSocket.OPEN)return;
  ws.send(JSON.stringify({{type:'interrupt',timestamp:Date.now()}}));
  setProcessing(false);
}}

sendBtn.addEventListener('click',send);
stopBtn.addEventListener('click',stop);
inputEl.addEventListener('keydown',function(e){{if(e.key==='Enter'&&!e.shiftKey){{e.preventDefault();send();}}}});
inputEl.addEventListener('input',function(){{inputEl.style.height='auto';inputEl.style.height=Math.min(inputEl.scrollHeight,120)+'px';}});

// Handle mobile keyboard resize
if(window.visualViewport){{
  var viewport=window.visualViewport;
  function onResize(){{
    var offset=window.innerHeight-viewport.height;
    document.body.style.height=viewport.height+'px';
    messagesEl.scrollTop=messagesEl.scrollHeight;
  }}
  viewport.addEventListener('resize',onResize);
  viewport.addEventListener('scroll',onResize);
}}

connect();
</script>
</body>
</html>"##,
        session_id = session_id,
        session_short = session_short,
        ws_url_json = ws_url_json,
    )
}
