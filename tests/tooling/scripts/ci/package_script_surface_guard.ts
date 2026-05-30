#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';
const ROOT=process.cwd();
type Violation={kind:string;path:string;detail:string};
function flag(name:string,fallback=''):string{const prefix=`--${name}=`;const direct=process.argv.slice(2).find(a=>a.startsWith(prefix));if(direct)return direct.slice(prefix.length);const idx=process.argv.indexOf(`--${name}`);return idx>=0?process.argv[idx+1]:fallback;}
function boolFlag(name:string,fallback=false):boolean{const raw=flag(name,fallback?'1':'0');return raw==='1'||raw==='true'}
function abs(rel:string):string{return path.join(ROOT,rel)}
function readJson(rel:string):any{return JSON.parse(fs.readFileSync(abs(rel),'utf8'))}
function read(rel:string):string{return fs.readFileSync(abs(rel),'utf8')}
function ensureDir(rel:string):void{fs.mkdirSync(path.dirname(abs(rel)),{recursive:true})}
function main():void{
 const strict=boolFlag('strict',true);const policyPath=flag('policy','validation/conformance/contracts/package_script_surface_policy.json');const outJson=flag('out-json','core/local/artifacts/package_script_surface_guard_current.json');const outMd=flag('out-markdown','local/workspace/reports/PACKAGE_SCRIPT_SURFACE_GUARD_CURRENT.md');
 const policy=readJson(policyPath);const pkg=readJson('package.json');const registry=readJson(policy.registry_path);const runner=read(policy.runner_path);const violations:Violation[]=[];const scripts=pkg.scripts||{};const ids=Object.keys(scripts);const regMap=new Map((registry.entries||[]).map((e:any)=>[String(e.id),String(e.command)]));
 if(policy.type!=='package_script_surface_policy')violations.push({kind:'policy_type_invalid',path:policyPath,detail:'Wrong type'});
 if(ids.length>Number(policy.baseline_script_count))violations.push({kind:'script_count_exceeds_baseline',path:'package.json',detail:`${ids.length} > ${policy.baseline_script_count}`});
 for(const id of policy.required_entrypoints||[])if(!scripts[id])violations.push({kind:'required_entrypoint_missing',path:'package.json',detail:String(id)});
 if(policy.policy?.registry_must_cover_all_package_scripts===true){
  for(const id of ids){if(!regMap.has(id))violations.push({kind:'registry_missing_script',path:policy.registry_path,detail:id});else if(regMap.get(id)!==scripts[id])violations.push({kind:'registry_command_mismatch',path:policy.registry_path,detail:id});}
  for(const entry of registry.entries||[])if(!scripts[entry.id])violations.push({kind:'registry_stale_script',path:policy.registry_path,detail:String(entry.id)});
 }else{
  const minimum=Number(policy.minimum_operator_registry_entries||0);
  if((registry.entries||[]).length<minimum)violations.push({kind:'operator_registry_below_minimum',path:policy.registry_path,detail:`${(registry.entries||[]).length} < ${minimum}`});
  for(const entry of registry.entries||[])if(!String(entry.command||'').trim())violations.push({kind:'registry_entry_missing_command',path:policy.registry_path,detail:String(entry.id)});
 }
 for(const token of ['command_registry_list','spawnSync','command_registry_groups'])if(!runner.includes(token))violations.push({kind:'runner_token_missing',path:policy.runner_path,detail:token});
 const payload={ok:violations.length===0,type:'package_script_surface_guard',generated_at:new Date().toISOString(),strict,script_count:ids.length,baseline_script_count:policy.baseline_script_count,registry_entry_count:(registry.entries||[]).length,registry_role:policy.policy?.compact_operator_registry_is_canonical?'compact_operator_surface':'package_script_mirror',violations};
 ensureDir(outJson);fs.writeFileSync(abs(outJson),JSON.stringify(payload,null,2)+'\n');ensureDir(outMd);fs.writeFileSync(abs(outMd),`# Package Script Surface Guard\n\n- ok: ${payload.ok}\n- script_count: ${ids.length}\n- baseline_script_count: ${policy.baseline_script_count}\n- registry_entry_count: ${payload.registry_entry_count}\n- violations: ${violations.length}\n\n${violations.map(v=>`- ${v.kind}: ${v.detail}`).join('\n')||'- none'}\n`);
 console.log(JSON.stringify(payload,null,2));if(strict&&!payload.ok)process.exit(1);
}
main();
