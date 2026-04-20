// ═══════════════════════════════════════════════════════════════════
//   OOZE — frontend analyzer logic (v8 event-focused)
// ═══════════════════════════════════════════════════════════════════

const API_BASE = window.location.origin;
const TOTAL_SUPPLY_SCALED = 1_000_000_000; // 1B tokens (standard pump.fun supply)

// ───── Helpers ─────
const fmtMoney = n => {
  if (n === undefined || n === null || isNaN(n)) return "$0";
  const a = Math.abs(n);
  if (a >= 1e6) return `$${(n/1e6).toFixed(2)}M`;
  if (a >= 1e3) return `$${(n/1e3).toFixed(1)}K`;
  return `$${n.toFixed(2)}`;
};
const fmtNum = n => {
  if (n === undefined || n === null || isNaN(n)) return "0";
  const a = Math.abs(n);
  if (a >= 1e6) return `${(n/1e6).toFixed(2)}M`;
  if (a >= 1e3) return `${(n/1e3).toFixed(1)}K`;
  return n.toFixed(0);
};
const fmtAge = h => {
  if (h < 1) return `${(h*60).toFixed(0)}m`;
  if (h < 24) return `${h.toFixed(1)}h`;
  return `${(h/24).toFixed(1)}d`;
};
const fmtEventTime = (eventMs, createdS) => {
  const eventS = Math.floor(eventMs / 1000);
  if (eventS <= createdS) return "launch";
  const diff = eventS - createdS;
  if (diff < 60) return `${diff}s`;
  if (diff < 3600) return `${Math.floor(diff/60)}m`;
  return `${(diff/3600).toFixed(1)}h`;
};
const short = (addr, n=10) => addr ? addr.slice(0, n) : "—";
const colorPct = p => {
  if (p == null) return "—";
  if (p > 0) return `<span class="green">+${p.toFixed(1)}%</span>`;
  if (p < 0) return `<span style="color:var(--red)">${p.toFixed(1)}%</span>`;
  return `${p.toFixed(1)}%`;
};
const esc = s => String(s).replace(/[&<>"']/g, c => ({
  "&":"&amp;","<":"&lt;",">":"&gt;",'"':"&quot;","'":"&#39;"
})[c]);

// ───── DOM ─────
const $mint = document.getElementById("mint");
const $runBtn = document.getElementById("run-btn");
const $status = document.getElementById("status");
const $report = document.getElementById("report");

// ───── Event bindings ─────
$runBtn.addEventListener("click", runAnalysis);
$mint.addEventListener("keydown", e => {
  if (e.key === "Enter") runAnalysis();
});
document.querySelectorAll(".example").forEach(a => {
  a.addEventListener("click", e => {
    e.preventDefault();
    $mint.value = a.dataset.mint;
    runAnalysis();
  });
});

// ───── Status helpers ─────
const GOO_LOADER = `
<div class="goo-wrap" aria-hidden="true">
  <div class="goo-scene">
    <div class="goo-blob goo-blob-big"></div>
    <div class="goo-blob goo-satellite"></div>
  </div>
</div>
`;

function setStatus(msg, isError=false) {
  $status.className = `status ${isError ? "error" : ""}`;
  $status.classList.remove("hidden");
  if (isError) {
    $status.innerHTML = msg;
  } else {
    $status.innerHTML = `${GOO_LOADER}<div class="goo-msg">${msg}</div>`;
  }
}
function hideStatus() { $status.classList.add("hidden"); }
function hideReport() { $report.classList.add("hidden"); $report.innerHTML = ""; }

// ───── Main flow ─────
async function runAnalysis() {
  const mint = $mint.value.trim();
  if (!mint) {
    setStatus("enter a token address", true);
    return;
  }
  if (mint.length < 32 || mint.length > 64) {
    setStatus("address looks wrong — expected 32-44 characters", true);
    return;
  }

  $runBtn.disabled = true;
  $runBtn.textContent = "ANALYZING";
  hideReport();

  const stages = [
    "fetching token overview",
    "pulling top traders",
    "loading minute-level price history",
    "detecting dramatic price events",
    "pulling trades at each event window",
    "detecting temporal clusters in each event",
    "replaying ordering under ooze",
  ];
  let stageIdx = 0;
  setStatus(stages[0]);
  const stageTimer = setInterval(() => {
    stageIdx = (stageIdx + 1) % stages.length;
    setStatus(stages[stageIdx]);
  }, 2200);

  try {
    const url = `${API_BASE}/api/analyze/${encodeURIComponent(mint)}`;
    const res = await fetch(url);
    clearInterval(stageTimer);

    if (!res.ok) {
      const err = await res.json().catch(() => ({ error: res.statusText }));
      throw new Error(err.error || `${res.status} ${res.statusText}`);
    }
    const report = await res.json();
    hideStatus();
    renderReport(report);
  } catch (e) {
    clearInterval(stageTimer);
    setStatus(`error: ${esc(e.message)}`, true);
  } finally {
    $runBtn.disabled = false;
    $runBtn.textContent = "ANALYZE";
  }
}

// ═══════════════════════════════════════════════════════════════════
//   REPORT RENDERING
// ═══════════════════════════════════════════════════════════════════

function renderReport(r) {
  const html = [
    renderVitals(r),
    renderEventsSummary(r),
    ...r.events.map((e, i) => renderEventDetail(e, i + 1, r)),
    renderVerdict(r),
    renderOozePitch(r),
  ].filter(Boolean).join("");

  $report.innerHTML = html;
  $report.classList.remove("hidden");
  $report.scrollIntoView({ behavior: "smooth", block: "start" });

  // Wire up cluster-toggle buttons
  $report.querySelectorAll(".cluster-toggle").forEach(btn => {
    const expandedLabel = btn.dataset.expanded || btn.innerHTML.replace("▼ SHOW", "▲ HIDE").replace(" MORE", "");
    const collapsedLabel = btn.innerHTML;
    btn.dataset.expanded = expandedLabel;
    btn.dataset.collapsed = collapsedLabel;

    btn.addEventListener("click", () => {
      const targetId = btn.dataset.target;
      const tbody = document.getElementById(targetId);
      if (!tbody) return;
      const isHidden = tbody.style.display === "none";
      tbody.style.display = isHidden ? "" : "none";
      btn.innerHTML = isHidden ? btn.dataset.expanded : btn.dataset.collapsed;
    });
  });
}

// ───── Vitals ─────
function renderVitals(r) {
  const t = r.overview.token;
  const p = r.primary_pool || {};
  const price = p.price?.usd ?? 0;
  // Backend uses camelCase for renamed fields: marketCap, tokenSupply
  const mcap = p.marketCap?.usd ?? p.market_cap?.usd ?? 0;
  const liquidity = p.liquidity?.usd ?? 0;
  const market = p.market ?? "unknown";
  const athMult = r.ath_mcap_usd > 0 && mcap > 0 ? (r.ath_mcap_usd / mcap) : null;

  const events = r.overview.events || {};
  const h1 = events.h1?.pct, h6 = events.h6?.pct, h24 = events.h24?.pct;

  const score = r.overview.risk?.score ?? 0;
  const scoreClass = score >= 7 ? "red" : score >= 4 ? "yellow" : "";
  const top10 = r.overview.risk?.top10 ?? 0;

  const risks = (r.overview.risk?.risks || []).map(rk => `
    <div class="risk-flag ${esc(rk.level)}">
      <span>${rk.level === "danger" ? "⛔" : "⚠"}</span>
      <span><b>${esc(rk.name)}</b>: ${esc(rk.description)}</span>
    </div>
  `).join("");

  return `
    <div class="section">
      <div class="section-title">▶ VITALS</div>
      <div style="margin-bottom:18px">
        <div style="font-size:20px;font-weight:bold;letter-spacing:2px">
          ${esc(t.name)} <span style="color:var(--ooze)">$${esc(t.symbol)}</span>
        </div>
        <div style="font-size:10px;opacity:0.4;letter-spacing:1px;margin-top:4px">${esc(t.mint)}</div>
      </div>

      <div class="kv"><div class="k">PRICE</div><div class="v">$${price.toFixed(8)}</div></div>
      <div class="kv"><div class="k">MARKET CAP (NOW)</div><div class="v">${fmtMoney(mcap)}</div></div>
      ${r.ath_mcap_usd > 0 ? `
      <div class="kv"><div class="k">ATH MARKET CAP</div><div class="v red">${fmtMoney(r.ath_mcap_usd)}${athMult ? ` <span style="opacity:0.6;font-weight:normal">(${athMult.toFixed(1)}x current)</span>` : ""}</div></div>` : ""}
      <div class="kv"><div class="k">LIQUIDITY</div><div class="v">${fmtMoney(liquidity)}</div></div>
      <div class="kv"><div class="k">AGE</div><div class="v">${fmtAge(r.age_hours)}</div></div>
      <div class="kv"><div class="k">HOLDERS</div><div class="v">${fmtNum(r.overview.holders)}</div></div>
      <div class="kv"><div class="k">PRIMARY VENUE</div><div class="v">${esc(market)}</div></div>

      <div class="kv"><div class="k">TRANSACTIONS</div><div class="v">${fmtNum(r.overview.txns)} (<span style="color:#66ff99">${fmtNum(r.overview.buys)} buys</span> / <span style="color:var(--red)">${fmtNum(r.overview.sells)} sells</span>)</div></div>

      <div class="kv"><div class="k">PRICE CHANGE</div><div class="v">
        ${h1 != null ? `1h ${colorPct(h1)}` : ""}
        ${h6 != null ? `· 6h ${colorPct(h6)}` : ""}
        ${h24 != null ? `· 24h ${colorPct(h24)}` : ""}
      </div></div>

      <div class="kv"><div class="k">RISK SCORE</div><div class="v ${scoreClass}">${score}/10</div></div>
      <div class="kv"><div class="k">TOP 10 HOLD</div><div class="v ${top10 > 50 ? "red" : "yellow"}">${top10.toFixed(2)}%</div></div>

      ${r.overview.risk?.rugged ? `<div class="kv"><div class="k">STATUS</div><div class="v red">RUGGED</div></div>` : ""}

      ${risks ? `<div style="margin-top:16px;font-size:11px;opacity:0.7;letter-spacing:2px">RISK FLAGS</div><div class="risk-flags">${risks}</div>` : ""}
    </div>
  `;
}

// ───── Events summary ─────
function renderEventsSummary(r) {
  if (!r.events || r.events.length === 0) {
    return `<div class="section"><div class="section-title">▶ DRAMATIC PRICE EVENTS</div>
      <p style="opacity:0.55">No significant price events detected.</p></div>`;
  }
  const createdS = r.overview.token.creation?.created_time ?? 0;

  const rows = r.events.map((e, i) => {
    const t = fmtEventTime(e.start_time_ms, createdS);
    const sevClass = e.severity === "DRAMATIC" ? "red" : e.severity === "MAJOR" ? "yellow" : "";
    const sevLabel = `<span class="badge ${sevClass}">${e.severity}</span>`;
    const typeLabel = e.event_type === "PUMP"
      ? `<span style="color:#66ff99;font-weight:bold">PUMP</span>`
      : `<span style="color:var(--red);font-weight:bold">DUMP</span>`;
    const coordStr = !e.trades_fetched
      ? `<span style="opacity:0.4">no trades</span>`
      : e.coordination_pct >= 30
        ? `<span style="color:var(--red);font-weight:bold">${e.coordination_pct.toFixed(0)}% clustered</span>`
        : e.coordination_pct >= 15
          ? `<span style="color:var(--yellow)">${e.coordination_pct.toFixed(0)}% clustered</span>`
          : `<span style="opacity:0.55">${e.coordination_pct.toFixed(0)}% clustered</span>`;
    return `<tr>
      <td class="num">${i + 1}</td>
      <td>${t}</td>
      <td>${sevLabel}</td>
      <td>${typeLabel}</td>
      <td class="num">${colorPct(e.price_change_pct)}</td>
      <td class="num">${e.trades.length}</td>
      <td class="num">${e.total_trade_sol.toFixed(2)} SOL</td>
      <td>${coordStr}</td>
    </tr>`;
  }).join("");

  return `
    <div class="section">
      <div class="section-title">▶ DRAMATIC PRICE EVENTS — SUMMARY</div>
      <div class="section-desc">
        Events detected by scanning minute-resolution price candles.
        <b style="color:var(--red)">DRAMATIC</b> ≥50% move · <b style="color:var(--yellow)">MAJOR</b> ≥25% · MINOR = biggest moves when token was calm.
        A trade is flagged as "clustered" if it shares a 2-second window with ≥2 other wallets trading the same direction.
        High clustering suggests coordinated execution (Jito bundles, bot swarms, or sandwich setups) rather than organic retail demand.
      </div>
      <table class="tbl">
        <tr><th>#</th><th>TIME</th><th>SEVERITY</th><th>TYPE</th><th style="text-align:right">Δ PRICE</th><th style="text-align:right">TRADES</th><th style="text-align:right">VOL</th><th>CLUSTERING</th></tr>
        ${rows}
      </table>
      <div style="margin-top:16px;padding-top:12px;border-top:1px solid var(--ooze-faint);font-size:12px">
        <b>${r.total_events_detected}</b> events ·
        <b style="color:var(--red)">${r.events_with_coordination}</b> with ≥30% clustered volume ·
        avg <b>${r.avg_coordination_pct.toFixed(0)}%</b> clustered volume across all events ·
        ${r.api_calls_used} API calls used
      </div>
    </div>
  `;
}

// ───── Single event detail ─────
function renderEventDetail(e, num, r) {
  const createdS = r.overview.token.creation?.created_time ?? 0;
  const sectionClass = e.severity === "DRAMATIC" ? "red" : e.severity === "MAJOR" ? "yellow" : "";
  const typeLabel = e.event_type === "PUMP"
    ? `<span style="color:#66ff99">PUMP</span>`
    : `<span style="color:var(--red)">DUMP</span>`;

  const coordClass = e.coordination_pct >= 30 ? "red"
    : e.coordination_pct >= 15 ? "yellow" : "";

  const headline = e.trades_fetched ? `
    <div style="margin-top:22px;padding:18px;background:rgba(0,0,0,0.4);border:1px solid var(--ooze-dim)">
      <div style="font-size:11px;letter-spacing:3px;opacity:0.55;margin-bottom:8px">HEADLINE</div>
      <div style="font-size:15px;line-height:1.6">
        <span class="v ${coordClass}" style="font-size:22px;font-weight:bold">
          ${e.coordination_pct.toFixed(0)}%
        </span>
        of this ${typeLabel}
        <b>${colorPct(e.price_change_pct)}</b> occurred as clustered trades from
        <b style="color:var(--red)">${e.coordinated_wallet_count} wallets</b>
        executing in tight 2-second windows
        (${e.coordinated_sol.toFixed(2)} of ${e.total_trade_sol.toFixed(2)} SOL).
      </div>
      ${e.coordination_pct >= 30 ? `
        <div style="margin-top:12px;font-size:12px;color:var(--red);letter-spacing:1px">
          ⚠ High clustering — this price move matches the signature of coordinated execution (Jito bundles, bot swarms, or sandwich setups) rather than independent retail activity.
        </div>` : e.coordination_pct >= 15 ? `
        <div style="margin-top:12px;font-size:12px;color:var(--yellow);letter-spacing:1px">
          ⓘ Moderate clustering — mixed signals. Some coordinated activity amid organic trading.
        </div>` : `
        <div style="margin-top:12px;font-size:12px;opacity:0.6;letter-spacing:1px">
          ✓ Low clustering — trades look largely independent and organic.
        </div>`}
    </div>
  ` : `
    <div style="margin-top:22px;padding:16px;opacity:0.55;font-size:12px;text-align:center;letter-spacing:2px">
      ⓘ No trades could be fetched for this event window.
    </div>
  `;

  // Sort clusters by volume (biggest first), show top 10 collapsed with expand toggle
  let clusters = "";
  if (e.clusters.length > 0) {
    const sortedClusters = [...e.clusters]
      .map((cl, origIdx) => ({ cl, origIdx }))
      .sort((a, b) => b.cl.total_sol - a.cl.total_sol);

    const TOP_N = 10;
    const topClusters = sortedClusters.slice(0, TOP_N);
    const restClusters = sortedClusters.slice(TOP_N);
    const hasMore = restClusters.length > 0;
    const clusterSectionId = `clusters-event-${num}`;

    const renderRow = ({ cl, origIdx }, displayIdx) => `
      <tr>
        <td class="num">${displayIdx + 1}</td>
        <td><span style="color:${cl.direction === "buy" ? "#66ff99" : "var(--red)"}">${cl.direction.toUpperCase()}</span></td>
        <td class="num">${cl.wallets.length}</td>
        <td class="num">${cl.total_sol.toFixed(2)}</td>
      </tr>
    `;

    clusters = `
      <div style="margin-top:16px;font-size:11px;letter-spacing:2px;opacity:0.65">
        CLUSTERS <span style="opacity:0.5">(top ${Math.min(TOP_N, e.clusters.length)} of ${e.clusters.length}, by volume)</span>
      </div>
      <table class="tbl">
        <tr><th>#</th><th>DIR</th><th style="text-align:right">WALLETS</th><th style="text-align:right">SOL</th></tr>
        ${topClusters.map((c, i) => renderRow(c, i)).join("")}
        ${hasMore ? `
          <tbody id="${clusterSectionId}" style="display:none">
            ${restClusters.map((c, i) => renderRow(c, i + TOP_N)).join("")}
          </tbody>
        ` : ""}
      </table>
      ${hasMore ? `
        <div style="text-align:center;margin-top:10px">
          <button
            class="cluster-toggle"
            data-target="${clusterSectionId}"
            style="background:transparent;border:1px solid var(--ooze-dim);color:var(--ooze);font-family:monospace;font-size:11px;letter-spacing:2px;padding:6px 14px;cursor:pointer;opacity:0.75">
            ▼ SHOW ${restClusters.length} MORE
          </button>
        </div>
      ` : ""}
    `;
  }

  const replay = e.ooze_replay ? renderEventReplay(e) : "";

  return `
    <div class="section ${sectionClass}">
      <div class="section-title">▶ EVENT #${num}: <span class="badge ${sectionClass}">${e.severity}</span> ${typeLabel} ${colorPct(e.price_change_pct)}</div>

      <div class="kv"><div class="k">WHEN</div><div class="v">${fmtEventTime(e.start_time_ms, createdS)} after launch (${e.candle_count} candle${e.candle_count === 1 ? "" : "s"})</div></div>
      <div class="kv"><div class="k">PRICE</div><div class="v">$${e.price_start.toFixed(8)} → $${e.price_end.toFixed(8)}</div></div>
      <div class="kv"><div class="k">RANGE</div><div class="v">$${e.price_low.toFixed(8)} (low) / $${e.price_high.toFixed(8)} (high)</div></div>
      <div class="kv"><div class="k">CANDLE VOLUME</div><div class="v">${e.candle_volume_sol.toFixed(2)} SOL</div></div>

      ${e.trades_fetched ? `
        <div class="kv"><div class="k">TRADES SAMPLED</div><div class="v">${e.trades.length}</div></div>
        <div class="kv"><div class="k">UNIQUE WALLETS</div><div class="v">${e.unique_wallets}</div></div>
        <div class="kv"><div class="k">WINDOW VOLUME</div><div class="v">${e.total_trade_sol.toFixed(2)} SOL</div></div>
        <div class="kv"><div class="k">CLUSTERS FOUND</div><div class="v">${e.clusters.length}</div></div>
        ${clusters}
      ` : ""}

      ${headline}
      ${replay}
    </div>
  `;
}

// ───── Ooze replay for one event ─────
function renderEventReplay(e) {
  const rep = e.ooze_replay;
  if (!rep) return "";

  const adjusted = Math.max(0, e.abs_magnitude - rep.price_impact_reduction);

  // Calculate aggregate dilution across top wallets (more meaningful than just #1)
  const comps = rep.wallet_comparisons || [];
  const topJitoSum = comps.reduce((s, w) => s + w.jito_tokens, 0);
  const topOozeSum = comps.reduce((s, w) => s + w.ooze_tokens, 0);
  const aggregateReduction = topJitoSum > 0 ? ((1 - topOozeSum / topJitoSum) * 100) : 0;
  const diluted = comps.filter(w => w.jito_tokens > 0 && (w.ooze_tokens / w.jito_tokens) < 0.99);
  const dilutedCount = diluted.length;

  // Build top wallets comparison table
  const walletsTable = comps.length > 0 ? `
    <div style="margin-top:20px">
      <div style="font-size:11px;letter-spacing:2px;opacity:0.65;margin-bottom:8px">
        TOP WALLETS — BEFORE &amp; AFTER
      </div>
      <table class="tbl">
        <tr>
          <th>WALLET</th>
          <th style="text-align:right">JITO (today)</th>
          <th style="text-align:right">OOZE (modeled)</th>
          <th style="text-align:right">Δ</th>
        </tr>
        ${comps.slice(0, 5).map(w => {
          const pct = w.jito_tokens > 0 ? ((w.ooze_tokens / w.jito_tokens - 1) * 100) : 0;
          const deltaColor = pct < -5 ? "var(--red)" : pct < -1 ? "var(--yellow)" : "";
          return `
            <tr>
              <td>${short(w.wallet, 8)}</td>
              <td class="num">${fmtNum(w.jito_tokens)}</td>
              <td class="num">${fmtNum(w.ooze_tokens)}</td>
              <td class="num" style="color:${deltaColor}">${pct >= 0 ? "+" : ""}${pct.toFixed(1)}%</td>
            </tr>
          `;
        }).join("")}
      </table>
    </div>
  ` : "";

  return `
    <div style="margin-top:22px;border-top:1px solid var(--ooze-dim);padding-top:18px">
      <div style="font-size:13px;letter-spacing:2px;color:var(--ooze);font-weight:bold;margin-bottom:10px">
        ▶ OOZE ORDERING SIMULATION
      </div>

      <div style="font-size:12px;opacity:0.75;line-height:1.6;margin-bottom:18px">
        <b>What this shows:</b> How the top 5 accumulating wallets would fare if this block had used
        Ooze's randomized ordering instead of Jito's bundle atomicity.
        Under Jito, bundled wallets land atomically and capture full intended volume.
        Under Ooze, wallets in the same 2-second window get randomized positions — later positions pay worse slippage.
        <br><br>
        <b>Note:</b> The single biggest accumulator is often a solo operator with no competing wallets in its windows —
        Ooze can't dilute a wallet operating alone. The real impact is on the <i>next tier</i> of competing wallets
        that were bundling together.
      </div>

      <div class="rp-compare">
        <div class="rp-side jito">
          <h4>JITO TODAY</h4>
          <div class="num">${fmtNum(topJitoSum)}</div>
          <div style="font-size:11px;opacity:0.75;margin-top:4px">tokens captured by top 5 wallets</div>
          <div style="font-size:11px;opacity:0.6;margin-top:2px">${(topJitoSum / TOTAL_SUPPLY_SCALED * 100).toFixed(2)}% of supply combined</div>
        </div>
        <div class="rp-side ooze">
          <h4>OOZE MODELED</h4>
          <div class="num">${fmtNum(topOozeSum)}</div>
          <div style="font-size:11px;opacity:0.75;margin-top:4px">tokens captured by same top 5 wallets</div>
          <div style="font-size:11px;opacity:0.6;margin-top:2px">${(topOozeSum / TOTAL_SUPPLY_SCALED * 100).toFixed(2)}% of supply combined</div>
        </div>
      </div>

      ${aggregateReduction > 1 ? `
        <div style="margin-top:10px;color:var(--ooze);font-weight:bold;letter-spacing:1px;font-size:13px">
          → Top 5 wallets collectively capture <b>${aggregateReduction.toFixed(1)}% FEWER</b> tokens under Ooze
          ${dilutedCount > 0 ? ` · ${dilutedCount} of 5 wallets diluted` : ""}
        </div>
      ` : `
        <div style="margin-top:10px;font-size:11px;opacity:0.55;letter-spacing:0.5px">
          ⓘ Top wallets were mostly operating alone in their trade windows — limited Ooze impact on this specific event.
        </div>
      `}

      ${walletsTable}

      <div style="margin-top:18px;padding:14px;background:rgba(0,255,85,0.05);border:1px solid var(--ooze-faint)">
        <div style="font-size:11px;letter-spacing:2px;opacity:0.75;margin-bottom:6px">PRICE IMPACT ESTIMATE</div>
        <div style="font-size:13px;line-height:1.6">
          This ${e.event_type.toLowerCase()} was <b>${e.abs_magnitude.toFixed(1)}%</b> on Jito today.
          With <b>${e.coordination_pct.toFixed(0)}%</b> of its volume from clustered/coordinated trades,
          Ooze would dilute roughly half that coordinated force.
          <br><br>
          → Under Ooze, modeled magnitude: <b style="color:var(--ooze);font-size:15px">${adjusted.toFixed(1)}%</b>
          <span style="opacity:0.6">(vs ${e.abs_magnitude.toFixed(1)}% actual) — reduction of ${rep.price_impact_reduction.toFixed(0)}%</span>
        </div>
      </div>

      <div style="margin-top:14px;padding:10px 12px;background:rgba(255,204,0,0.06);border-left:2px solid var(--yellow);font-size:11px;line-height:1.6;opacity:0.85">
        <b style="color:var(--yellow);letter-spacing:1px">⚠ IMPORTANT CAVEAT</b><br>
        This model assumes bundles still fire under Ooze, just with randomized intra-block ordering.
        In reality, Ooze may disrupt bundle-based strategies <i>entirely</i> — coordinated wallets
        that rely on atomic execution (sandwich attacks, precise pump-and-dump timing) might simply
        not attempt the strategy if atomicity isn't guaranteed. The actual impact on these price events
        could be substantially larger than modeled — some events may not happen at all.
      </div>

      <div style="margin-top:14px;font-size:10px;opacity:0.45;line-height:1.7">
        ${(rep.notes || []).map(n => `ⓘ ${esc(n)}`).join("<br>")}
      </div>
    </div>
  `;
}

// ───── Verdict ─────
function renderVerdict(r) {
  const total = r.total_events_detected;
  const heavy = r.events_with_coordination;

  if (total === 0) {
    return `<div class="verdict"><div class="label">VERDICT</div><div class="text">INSUFFICIENT DATA</div></div>`;
  }

  let text, cls, sub;
  if (heavy >= Math.ceil(total / 2)) {
    text = "MANUFACTURED PRICE ACTION";
    cls = "danger";
    sub = `${heavy} of ${total} events show high clustering — signature of coordinated execution`;
  } else if (heavy > 0) {
    text = "MIXED — CLUSTERING PRESENT";
    cls = "warning";
    sub = `${heavy} of ${total} events show significant clustering`;
  } else {
    text = "MOSTLY ORGANIC";
    cls = "";
    sub = "No events showed significant clustering — trades look independent";
  }

  return `
    <div class="verdict ${cls}">
      <div class="label">VERDICT</div>
      <div class="text">${text}</div>
      <div style="margin-top:16px;font-size:12px;opacity:0.75">${sub}</div>
      <div style="margin-top:12px;font-size:11px;opacity:0.55;letter-spacing:1px">
        avg clustered volume across events: ${r.avg_coordination_pct.toFixed(0)}% · ${r.api_calls_used} API calls used
      </div>
      <div style="margin-top:18px;padding-top:14px;border-top:1px solid var(--ooze-faint);font-size:10px;opacity:0.55;line-height:1.5;letter-spacing:0.5px;text-align:left">
        <b>Methodology note:</b> "Clustered" = trades from ≥3 distinct wallets in the same direction within a 2-second window. This captures Jito bundles, bot swarms, and sandwich setups — not just atomic bundles. It's a statistical proxy for coordinated execution, not a bundle-hash forensic. Tight clustering on this scale is extremely improbable for independent retail activity.
      </div>
    </div>
  `;
}

// ───── Ooze pitch ─────
function renderOozePitch(r) {
  return `
    <div class="section">
      <div class="section-title">▶ WHAT OOZE CHANGES</div>
      <div style="margin-bottom:18px;font-size:11px;letter-spacing:2px;color:var(--red)">UNDER JITO TODAY:</div>
      <ul style="list-style:none;padding:0;margin:0;font-size:13px">
        <li style="padding:4px 0">— Coordinated bundles execute atomically; a wallet group can capture best prices at dramatic moments</li>
        ${r.events_with_coordination > 0 ? `<li style="padding:4px 0">— ${r.events_with_coordination} of the ${r.total_events_detected} dramatic events on this token show clustering patterns consistent with coordinated execution</li>` : ""}
      </ul>

      <div style="margin:20px 0 18px;font-size:11px;letter-spacing:2px;color:var(--ooze)">UNDER OOZE:</div>
      <ul style="list-style:none;padding:0;margin:0;font-size:13px">
        <li style="padding:4px 0">— Multi-wallet bundles cannot execute as a unit at the same price</li>
        <li style="padding:4px 0">— Retail transactions interleave with coordinated orders</li>
        <li style="padding:4px 0">— Coordinated price impact gets diluted — fewer one-sided pumps/dumps</li>
        <li style="padding:4px 0">— Validators still earn priority fees — just fairly</li>
      </ul>

      <div style="text-align:center;margin-top:25px;font-style:italic;opacity:0.65;letter-spacing:1px">
        Jito is not evil. Monopoly is. Ooze is an alternative.
      </div>
    </div>
  `;
}