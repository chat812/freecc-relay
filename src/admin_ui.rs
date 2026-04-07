pub fn render_admin_login() -> String {
    r#"<!DOCTYPE html><html><head><title>Admin Login</title>
<meta name="viewport" content="width=device-width, initial-scale=1">
<style>body{background:#1a1b26;color:#c0caf5;font-family:monospace;display:flex;align-items:center;justify-content:center;height:100vh;margin:0;}
.box{background:#24283b;border:1px solid #3b4261;border-radius:8px;padding:30px;text-align:center;}
h2{color:#7aa2f7;margin-bottom:16px;}
p{color:#565f89;font-size:12px;margin-top:12px;}</style></head>
<body><div class="box"><h2>Admin</h2>
<p>Access the admin dashboard via the URL shown in server logs.</p>
</div></body></html>"#.to_string()
}

pub fn render_admin_ui(admin_token: &str) -> String {
    let token_json = serde_json::to_string(admin_token).unwrap();
    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Free CC Relay - Admin</title>
<style>
  * {{ margin: 0; padding: 0; box-sizing: border-box; }}
  body {{ background: #1a1b26; color: #c0caf5; font-family: 'SF Mono', 'Cascadia Code', monospace; padding: 20px; }}
  h1 {{ color: #7aa2f7; font-size: 18px; margin-bottom: 20px; display: flex; align-items: center; gap: 12px; }}
  h2 {{ color: #7aa2f7; font-size: 14px; margin: 20px 0 10px; }}
  .card {{ background: #24283b; border: 1px solid #3b4261; border-radius: 8px; padding: 12px 14px; margin-bottom: 8px; }}
  .row {{ display: flex; gap: 10px; align-items: center; flex-wrap: wrap; }}
  .label {{ color: #565f89; font-size: 11px; text-transform: uppercase; letter-spacing: 0.5px; }}
  .value {{ font-size: 13px; }}
  .status {{ display: inline-block; padding: 2px 8px; border-radius: 10px; font-size: 11px; }}
  .active {{ background: rgba(158,206,106,0.15); color: #9ece6a; }}
  .waiting {{ background: rgba(224,175,104,0.15); color: #e0af68; }}
  .btn {{ background: #3b4261; color: #c0caf5; border: 1px solid #565f89; padding: 5px 14px; border-radius: 4px; cursor: pointer; font-family: inherit; font-size: 12px; }}
  .btn:hover {{ background: #565f89; }}
  .btn-danger {{ border-color: #f7768e; color: #f7768e; }}
  .btn-danger:hover {{ background: rgba(247,118,142,0.2); }}
  .btn-primary {{ background: #7aa2f7; color: #1a1b26; border-color: #7aa2f7; }}
  .btn-primary:hover {{ opacity: 0.9; }}
  .btn-approve {{ background: #9ece6a; color: #1a1b26; border-color: #9ece6a; font-weight: bold; }}
  .btn-approve:hover {{ opacity: 0.9; }}
  .card-warn {{ border-color: #e0af68; }}
  .pending {{ background: rgba(224,175,104,0.25); color: #e0af68; animation: pulse 2s infinite; }}
  .rejected {{ background: rgba(247,118,142,0.15); color: #f7768e; }}
  .approved {{ background: rgba(158,206,106,0.15); color: #9ece6a; }}
  @keyframes pulse {{ 0%,100% {{ opacity: 1; }} 50% {{ opacity: 0.6; }} }}
  input.name-input {{ background: #1a1b26; border: 1px solid #3b4261; color: #c0caf5; padding: 4px 8px; border-radius: 4px; font-family: inherit; font-size: 12px; width: 120px; }}
  input.name-input:focus {{ border-color: #7aa2f7; outline: none; }}
  .stat-warn {{ color: #e0af68; }}
  .stats {{ display: flex; gap: 16px; margin-bottom: 20px; flex-wrap: wrap; }}
  .stat {{ background: #24283b; border: 1px solid #3b4261; border-radius: 8px; padding: 12px 20px; text-align: center; min-width: 100px; }}
  .stat-val {{ font-size: 24px; font-weight: bold; color: #7aa2f7; }}
  .stat-label {{ font-size: 11px; color: #565f89; margin-top: 4px; }}
  .empty {{ color: #565f89; font-style: italic; padding: 20px; text-align: center; }}
  .toolbar {{ display: flex; gap: 8px; margin-bottom: 12px; align-items: center; flex-wrap: wrap; }}
  .toolbar .spacer {{ flex: 1; }}
  .check {{ width: 16px; height: 16px; accent-color: #7aa2f7; cursor: pointer; }}
  .selected-count {{ color: #7aa2f7; font-size: 12px; }}
  .group {{ background: #24283b; border: 1px solid #3b4261; border-radius: 8px; margin-bottom: 10px; overflow: hidden; }}
  .group-header {{ display: flex; align-items: center; gap: 10px; padding: 10px 14px; cursor: pointer; user-select: none; }}
  .group-header:hover {{ background: rgba(122,162,247,0.06); }}
  .group-toggle {{ font-size: 10px; color: #565f89; transition: transform 0.15s; display: inline-block; }}
  .group-toggle.open {{ transform: rotate(90deg); }}
  .group-name {{ font-weight: 600; font-size: 13px; color: #7aa2f7; }}
  .group-badge {{ font-size: 11px; padding: 1px 7px; border-radius: 10px; background: rgba(122,162,247,0.15); color: #7aa2f7; }}
  .group-badge-active {{ background: rgba(158,206,106,0.15); color: #9ece6a; }}
  .group-summary {{ font-size: 11px; color: #565f89; margin-left: auto; }}
  .group-body {{ display: none; border-top: 1px solid #3b4261; }}
  .group-body.open {{ display: block; }}
  .group-body .card {{ border-radius: 0; border-left: none; border-right: none; border-bottom: none; margin-bottom: 0; }}
  .group-body .card:last-child {{ border-bottom: none; }}
  .toast {{ position: fixed; bottom: 20px; right: 20px; background: #24283b; border: 1px solid #3b4261; color: #9ece6a; padding: 10px 16px; border-radius: 6px; font-size: 13px; display: none; z-index: 100; }}
  select {{ background: #1a1b26; border: 1px solid #3b4261; color: #c0caf5; padding: 5px 8px; border-radius: 4px; font-family: inherit; font-size: 12px; }}
</style>
</head>
<body>
<h1>Free CC Relay - Admin <span style="flex:1"></span>
  <button class="btn" onclick="load()">Refresh</button>
</h1>
<div class="stats">
  <div class="stat"><div class="stat-val" id="total">-</div><div class="stat-label">Sessions</div></div>
  <div class="stat"><div class="stat-val" id="active-count">-</div><div class="stat-label">Active</div></div>
  <div class="stat"><div class="stat-val" id="waiting-count">-</div><div class="stat-label">Waiting</div></div>
  <div class="stat"><div class="stat-val" id="pairing-count">-</div><div class="stat-label">Pairing</div></div>
  <div class="stat"><div class="stat-val" id="client-count">-</div><div class="stat-label">Clients</div></div>
</div>

<!-- Pairing Requests -->
<div id="pairing-section" style="display:none">
<h2>Pairing Requests</h2>
<div id="pairings"></div>
</div>

<h2>Sessions</h2>
<div class="toolbar">
  <label style="cursor:pointer;font-size:12px;"><input type="checkbox" class="check" id="select-all" onchange="toggleAll()"> Select all</label>
  <span class="selected-count" id="selected-count"></span>
  <span class="spacer"></span>
  <button class="btn btn-danger" id="btn-kill" onclick="killSelected()" style="display:none">Kill selected</button>
  <button class="btn" onclick="cleanupOld()">Cleanup old</button>
  <select id="cleanup-age">
    <option value="1">Older than 1h</option>
    <option value="6">Older than 6h</option>
    <option value="24" selected>Older than 24h</option>
    <option value="168">Older than 7d</option>
  </select>
  <button class="btn btn-danger" onclick="killAll()">Kill all</button>
</div>
<div id="sessions"><div class="empty">Loading...</div></div>
<div class="toast" id="toast"></div>
<script>
var adminToken={token_json};
var allSessions=[];
var expandedGroups={{}};
function toggleGroup(name){{
  expandedGroups[name]=!expandedGroups[name];
  var body=document.getElementById('group-'+name);
  var header=body.previousElementSibling;
  var toggle=header.querySelector('.group-toggle');
  if(expandedGroups[name]){{body.classList.add('open');toggle.classList.add('open');}}
  else{{body.classList.remove('open');toggle.classList.remove('open');}}
}}
function timeAgo(ts){{var s=Math.floor((Date.now()-ts)/1000);if(s<60)return s+'s ago';if(s<3600)return Math.floor(s/60)+'m ago';if(s<86400)return Math.floor(s/3600)+'h ago';return Math.floor(s/86400)+'d ago';}}
function toast(msg){{var el=document.getElementById('toast');el.textContent=msg;el.style.display='block';setTimeout(function(){{el.style.display='none';}},3000);}}
function adminFetch(url,opts){{opts=opts||{{}};opts.headers=opts.headers||{{}};opts.headers['X-Admin-Token']=adminToken;if(opts.body&&typeof opts.body==='object'){{opts.headers['Content-Type']='application/json';opts.body=JSON.stringify(opts.body);}}return fetch(url,opts).then(function(r){{return r.json();}});}}
function getSelected(){{var checks=document.querySelectorAll('.session-check:checked');var ids=[];checks.forEach(function(c){{ids.push(c.dataset.id);}});return ids;}}
function updateSelectedCount(){{var ids=getSelected();var el=document.getElementById('selected-count');var btn=document.getElementById('btn-kill');if(ids.length>0){{el.textContent=ids.length+' selected';btn.style.display='inline-block';}}else{{el.textContent='';btn.style.display='none';}}}}
function toggleAll(){{var checked=document.getElementById('select-all').checked;document.querySelectorAll('.session-check').forEach(function(c){{c.checked=checked;}});updateSelectedCount();}}
function killSelected(){{var ids=getSelected();if(ids.length===0)return;if(!confirm('Kill '+ids.length+' session(s)?'))return;adminFetch('/api/admin/sessions/kill',{{method:'POST',body:{{sessionIds:ids}}}}).then(function(r){{toast('Killed '+r.closed+' session(s)');load();}});}}
function killOne(id){{if(!confirm('Kill this session?'))return;adminFetch('/api/admin/sessions/kill',{{method:'POST',body:{{sessionIds:[id]}}}}).then(function(r){{toast('Session killed');load();}});}}
function killAll(){{if(!confirm('Kill ALL sessions?'))return;adminFetch('/api/admin/sessions/kill-all',{{method:'POST'}}).then(function(r){{toast('Killed '+r.closed+' session(s)');load();}});}}
function cleanupOld(){{var hours=parseInt(document.getElementById('cleanup-age').value);adminFetch('/api/admin/sessions/cleanup',{{method:'POST',body:{{maxAgeHours:hours}}}}).then(function(r){{toast('Removed '+r.removed+' old session(s)');load();}});}}
function approvePairing(id){{
  var nameInput=document.getElementById('name-'+id);
  var name=nameInput?nameInput.value.trim():'';
  adminFetch('/api/admin/pairings/'+id+'/approve',{{method:'POST',body:{{name:name}}}})
    .then(function(r){{toast('Approved: '+(r.clientName||''));load();}});
}}
function rejectPairing(id){{
  adminFetch('/api/admin/pairings/'+id+'/reject',{{method:'POST'}})
    .then(function(){{toast('Rejected');load();}});
}}
function load(){{
  Promise.all([
    adminFetch('/api/admin/sessions'),
    adminFetch('/api/admin/pairings')
  ]).then(function(results){{
    var data=results[0];
    var pairingData=results[1];
    allSessions=data.sessions;
    document.getElementById('total').textContent=data.sessions.length;
    document.getElementById('active-count').textContent=data.sessions.filter(function(s){{return s.status==='active';}}).length;
    document.getElementById('waiting-count').textContent=data.sessions.filter(function(s){{return s.status==='waiting';}}).length;
    document.getElementById('client-count').textContent=data.clients.length;
    var pendingPairings=pairingData.pairings.filter(function(p){{return p.status==='pending';}});
    var pairingCountEl=document.getElementById('pairing-count');
    pairingCountEl.textContent=pendingPairings.length;
    pairingCountEl.className=pendingPairings.length>0?'stat-val stat-warn':'stat-val';
    var pairingSection=document.getElementById('pairing-section');
    var pairingsEl=document.getElementById('pairings');
    if(pairingData.pairings.length>0){{
      pairingSection.style.display='block';
      var phtml='';
      for(var j=0;j<pairingData.pairings.length;j++){{
        var p=pairingData.pairings[j];
        var cls=p.status==='pending'?'card card-warn':'card';
        phtml+='<div class="'+cls+'"><div class="row">';
        phtml+='<span class="status '+p.status+'">'+p.status+'</span>';
        phtml+='<span class="label">host:</span><span class="value">'+p.hostname+'</span>';
        phtml+='<span class="label">ip:</span><span class="value">'+p.ip+'</span>';
        phtml+='<span class="label">time:</span><span class="value">'+timeAgo(p.createdAt)+'</span>';
        if(p.status==='pending'){{
          phtml+='<span style="flex:1"></span>';
          phtml+='<input type="text" class="name-input" id="name-'+p.id+'" placeholder="client name" value="'+p.hostname+'">';
          phtml+='<button class="btn btn-approve" onclick="approvePairing(\''+p.id+'\')">Approve</button>';
          phtml+='<button class="btn btn-danger" onclick="rejectPairing(\''+p.id+'\')">Reject</button>';
        }}
        if(p.status==='approved'){{
          phtml+='<span class="label">name:</span><span class="value">'+p.clientName+'</span>';
        }}
        phtml+='</div></div>';
      }}
      pairingsEl.innerHTML=phtml;
    }}else{{
      pairingSection.style.display='none';
    }}
    document.getElementById('select-all').checked=false;
    var el=document.getElementById('sessions');
    if(data.sessions.length===0){{el.innerHTML='<div class="empty">No sessions</div>';updateSelectedCount();return;}}
    data.sessions.sort(function(a,b){{return b.lastActivity-a.lastActivity;}});
    var groups={{}};var groupOrder=[];
    for(var i=0;i<data.sessions.length;i++){{
      var s=data.sessions[i];var name=s.clientName||'unknown';
      if(!groups[name]){{groups[name]=[];groupOrder.push(name);}}
      groups[name].push(s);
    }}
    var html='';
    for(var g=0;g<groupOrder.length;g++){{
      var name=groupOrder[g];var items=groups[name];
      var activeCount=items.filter(function(s){{return s.status==='active';}}).length;
      var totalMsgs=items.reduce(function(sum,s){{return sum+s.messageCount;}},0);
      var wasOpen=expandedGroups[name];
      var openCls=wasOpen?' open':'';
      html+='<div class="group">';
      html+='<div class="group-header" onclick="toggleGroup(\''+name+'\')">';
      html+='<span class="group-toggle'+openCls+'">\u25B6</span>';
      html+='<span class="group-name">'+name+'</span>';
      html+='<span class="group-badge">'+items.length+' session'+(items.length!==1?'s':'')+'</span>';
      if(activeCount>0)html+='<span class="group-badge group-badge-active">'+activeCount+' active</span>';
      html+='<span class="group-summary">'+totalMsgs+' msgs \u00B7 '+timeAgo(items[0].lastActivity)+'</span>';
      html+='</div>';
      html+='<div class="group-body'+openCls+'" id="group-'+name+'">';
      for(var i=0;i<items.length;i++){{
        var s=items[i];var statusCls=s.status==='active'?'active':'waiting';
        html+='<div class="card"><div class="row">';
        html+='<input type="checkbox" class="check session-check" data-id="'+s.id+'" onchange="updateSelectedCount()">';
        html+='<span class="status '+statusCls+'">'+s.status+'</span>';
        html+='<span class="value"><strong>'+s.id.slice(0,16)+'...</strong></span>';
        html+='<span class="label">cli:</span><span class="value">'+(s.hasCli?'\u2705':'\u274C')+'</span>';
        html+='<span class="label">web:</span><span class="value">'+s.webClients+'</span>';
        html+='<span class="label">msgs:</span><span class="value">'+s.messageCount+'</span>';
        html+='<span class="label">active:</span><span class="value">'+timeAgo(s.lastActivity)+'</span>';
        html+='<span style="flex:1"></span>';
        html+='<button class="btn btn-danger" onclick="killOne(\''+s.id+'\')">Kill</button>';
        html+='</div></div>';
      }}
      html+='</div></div>';
    }}
    el.innerHTML=html;updateSelectedCount();
  }});
}}
load();setInterval(load,5000);
</script>
</body>
</html>"##,
        token_json = token_json,
    )
}
