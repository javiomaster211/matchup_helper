// MatchupHelper - Frontend Application

const { invoke } = window.__TAURI__.core;
const { getCurrentWindow } = window.__TAURI__.window;

// Data Dragon configuration - will be loaded dynamically
let DDRAGON_VERSION = "16.2.1"; // Default fallback for Season 16
let DDRAGON_BASE = `https://ddragon.leagueoflegends.com/cdn/${DDRAGON_VERSION}`;

// Fetch latest Data Dragon version
async function fetchLatestVersion() {
  try {
    const response = await fetch('https://ddragon.leagueoflegends.com/api/versions.json');
    const versions = await response.json();
    if (versions && versions.length > 0) {
      DDRAGON_VERSION = versions[0];
      DDRAGON_BASE = `https://ddragon.leagueoflegends.com/cdn/${DDRAGON_VERSION}`;
      console.log('Using Data Dragon version:', DDRAGON_VERSION);
    }
  } catch (error) {
    console.warn('Could not fetch latest version, using default:', DDRAGON_VERSION);
  }
}

// State
let state = {
  matchups: [],
  matches: [],
  champions: [],
  currentMatchup: null,
  currentMatch: null,
  filters: {
    myChampion: '',
    enemyChampion: '',
    role: '',
    tags: [],
    search: ''
  },
  alwaysOnTop: false,
  lcuConnected: false
};

// DOM Elements
const elements = {
  tabs: document.querySelectorAll('.tab'),
  tabContents: document.querySelectorAll('.tab-content'),
  searchInput: document.getElementById('search-input'),
  filterMyChampion: document.getElementById('filter-my-champion'),
  filterEnemyChampion: document.getElementById('filter-enemy-champion'),
  filterRole: document.getElementById('filter-role'),
  tagFilters: document.getElementById('tag-filters'),
  matchupsList: document.getElementById('matchups-list'),
  historyList: document.getElementById('history-list'),
  btnNewMatchup: document.getElementById('btn-new-matchup'),
  btnAlwaysOnTop: document.getElementById('btn-always-on-top'),
  btnConnectLcu: document.getElementById('btn-connect-lcu'),
  lcuStatus: document.getElementById('lcu-status'),
  modalMatchup: document.getElementById('modal-matchup'),
  btnCloseModal: document.getElementById('btn-close-modal'),
  btnSaveMatchup: document.getElementById('btn-save-matchup'),
  btnDeleteMatchup: document.getElementById('btn-delete-matchup'),
  detailMyChampionIcon: document.getElementById('detail-my-champion-icon'),
  detailEnemyChampionIcon: document.getElementById('detail-enemy-champion-icon'),
  detailTitle: document.getElementById('detail-title'),
  detailRole: document.getElementById('detail-role'),
  detailTags: document.getElementById('detail-tags'),
  inputNewTag: document.getElementById('input-new-tag'),
  detailRunes: document.getElementById('detail-runes'),
  detailSummoners: document.getElementById('detail-summoners'),
  detailItems: document.getElementById('detail-items'),
  detailNotes: document.getElementById('detail-notes'),
  detailVersion: document.getElementById('detail-version'),
  detailVersionDate: document.getElementById('detail-version-date'),
  modalNewMatchup: document.getElementById('modal-new-matchup'),
  btnCloseNewModal: document.getElementById('btn-close-new-modal'),
  btnCancelNew: document.getElementById('btn-cancel-new'),
  btnCreateMatchup: document.getElementById('btn-create-matchup'),
  newMyChampion: document.getElementById('new-my-champion'),
  newEnemyChampion: document.getElementById('new-enemy-champion'),
  newMyChampionDropdown: document.getElementById('new-my-champion-dropdown'),
  newEnemyChampionDropdown: document.getElementById('new-enemy-champion-dropdown'),
  newRole: document.getElementById('new-role'),
  modalMatch: document.getElementById('modal-match'),
  btnCloseMatchModal: document.getElementById('btn-close-match-modal'),
  btnSaveMatch: document.getElementById('btn-save-match'),
  matchMyChampionIcon: document.getElementById('match-my-champion-icon'),
  matchEnemyChampionIcon: document.getElementById('match-enemy-champion-icon'),
  matchResult: document.getElementById('match-result'),
  matchDate: document.getElementById('match-date'),
  matchNotes: document.getElementById('match-notes'),
  matchLinkedMatchup: document.getElementById('match-linked-matchup')
};

// ==================== Initialization ====================

async function init() {
  try {
    await fetchLatestVersion();
    await loadChampions();
    await loadMatchups();
    await loadMatches();
    setupEventListeners();
    populateChampionFilters();
    renderMatchups();
    console.log('MatchupHelper initialized');
  } catch (error) {
    console.error('Initialization error:', error);
  }
}

// ==================== Data Loading ====================

async function loadChampions() {
  try {
    const response = await fetch(`${DDRAGON_BASE}/data/en_US/champion.json`);
    const data = await response.json();
    state.champions = Object.keys(data.data).sort();
    console.log(`Loaded ${state.champions.length} champions`);
  } catch (error) {
    console.error('Error loading champions:', error);
    state.champions = ['Aatrox', 'Ahri', 'Akali', 'Darius', 'Garen', 'Lux', 'Yasuo', 'Zed'];
  }
}

async function loadMatchups() {
  try {
    const filter = buildFilter();
    state.matchups = await invoke('get_matchups', { filter });
  } catch (error) {
    console.error('Error loading matchups:', error);
    state.matchups = [];
  }
}

async function loadMatches() {
  try {
    state.matches = await invoke('get_matches', {});
  } catch (error) {
    console.error('Error loading matches:', error);
    state.matches = [];
  }
}

// ==================== Event Listeners ====================

function setupEventListeners() {
  elements.tabs.forEach(tab => {
    tab.addEventListener('click', () => switchTab(tab.dataset.tab));
  });

  elements.searchInput.addEventListener('input', debounce(handleSearch, 300));
  elements.filterMyChampion.addEventListener('change', handleFilterChange);
  elements.filterEnemyChampion.addEventListener('change', handleFilterChange);
  elements.filterRole.addEventListener('change', handleFilterChange);

  elements.btnNewMatchup.addEventListener('click', openNewMatchupModal);
  elements.btnCloseNewModal.addEventListener('click', closeNewMatchupModal);
  elements.btnCancelNew.addEventListener('click', closeNewMatchupModal);
  elements.btnCreateMatchup.addEventListener('click', createMatchup);

  setupChampionSearch(elements.newMyChampion, elements.newMyChampionDropdown);
  setupChampionSearch(elements.newEnemyChampion, elements.newEnemyChampionDropdown);

  elements.btnCloseModal.addEventListener('click', closeMatchupModal);
  elements.btnSaveMatchup.addEventListener('click', saveMatchup);
  elements.btnDeleteMatchup.addEventListener('click', deleteMatchup);
  elements.inputNewTag.addEventListener('keypress', handleAddTag);
  elements.detailVersion.addEventListener('change', handleVersionChange);

  elements.btnCloseMatchModal.addEventListener('click', closeMatchModal);
  elements.btnSaveMatch.addEventListener('click', saveMatch);

  elements.btnAlwaysOnTop.addEventListener('click', toggleAlwaysOnTop);
  elements.btnConnectLcu.addEventListener('click', connectToLcu);

  document.addEventListener('keydown', handleGlobalShortcuts);

  elements.modalMatchup.addEventListener('click', (e) => {
    if (e.target === elements.modalMatchup) closeMatchupModal();
  });
  elements.modalNewMatchup.addEventListener('click', (e) => {
    if (e.target === elements.modalNewMatchup) closeNewMatchupModal();
  });
  elements.modalMatch.addEventListener('click', (e) => {
    if (e.target === elements.modalMatch) closeMatchModal();
  });
}

function setupChampionSearch(input, dropdown) {
  input.addEventListener('input', () => {
    const query = input.value.toLowerCase();
    if (query.length < 1) {
      dropdown.classList.add('hidden');
      return;
    }

    const filtered = state.champions.filter(c =>
      c.toLowerCase().includes(query)
    ).slice(0, 10);

    if (filtered.length === 0) {
      dropdown.classList.add('hidden');
      return;
    }

    dropdown.innerHTML = filtered.map(champion => `
      <div class="champion-option" data-champion="${champion}">
        <img src="${getChampionIcon(champion)}" alt="${champion}">
        <span>${champion}</span>
      </div>
    `).join('');

    dropdown.classList.remove('hidden');

    dropdown.querySelectorAll('.champion-option').forEach(option => {
      option.addEventListener('click', () => {
        input.value = option.dataset.champion;
        dropdown.classList.add('hidden');
      });
    });
  });

  document.addEventListener('click', (e) => {
    if (!input.contains(e.target) && !dropdown.contains(e.target)) {
      dropdown.classList.add('hidden');
    }
  });
}

// ==================== Tab Navigation ====================

function switchTab(tabName) {
  elements.tabs.forEach(tab => {
    tab.classList.toggle('active', tab.dataset.tab === tabName);
  });

  elements.tabContents.forEach(content => {
    content.classList.toggle('active', content.id === `tab-${tabName}`);
  });

  if (tabName === 'history') {
    renderHistory();
  }
}

// ==================== Filtering ====================

function buildFilter() {
  const filter = {};
  if (state.filters.myChampion) filter.my_champion = state.filters.myChampion;
  if (state.filters.enemyChampion) filter.enemy_champion = state.filters.enemyChampion;
  if (state.filters.role) filter.role = state.filters.role;
  if (state.filters.tags.length > 0) filter.tags = state.filters.tags;
  if (state.filters.search) filter.search = state.filters.search;
  return Object.keys(filter).length > 0 ? filter : null;
}

async function handleSearch(e) {
  state.filters.search = e.target.value;
  await loadMatchups();
  renderMatchups();
}

async function handleFilterChange() {
  state.filters.myChampion = elements.filterMyChampion.value;
  state.filters.enemyChampion = elements.filterEnemyChampion.value;
  state.filters.role = elements.filterRole.value;
  await loadMatchups();
  renderMatchups();
}

function populateChampionFilters() {
  const uniqueMyChampions = [...new Set(state.matchups.map(m => m.my_champion))].sort();
  const uniqueEnemyChampions = [...new Set(state.matchups.map(m => m.enemy_champion))].sort();

  elements.filterMyChampion.innerHTML = '<option value="">My Champion</option>' +
    uniqueMyChampions.map(c => `<option value="${c}">${c}</option>`).join('');

  elements.filterEnemyChampion.innerHTML = '<option value="">Enemy Champion</option>' +
    uniqueEnemyChampions.map(c => `<option value="${c}">${c}</option>`).join('');
}

// ==================== Rendering ====================

function renderMatchups() {
  if (state.matchups.length === 0) {
    elements.matchupsList.innerHTML = `
      <div class="empty-state">
        <h3>No matchups yet</h3>
        <p>Create your first matchup with the "+ New" button</p>
      </div>
    `;
    return;
  }

  elements.matchupsList.innerHTML = state.matchups.map(matchup => {
    const currentVersion = matchup.versions[matchup.current_version - 1] || matchup.versions[0];
    const tags = currentVersion?.tags || [];

    return `
      <div class="matchup-card" data-id="${matchup.id}">
        <div class="matchup-champions">
          <img class="champion-icon" src="${getChampionIcon(matchup.my_champion)}" alt="${matchup.my_champion}">
          <span class="vs">vs</span>
          <img class="champion-icon" src="${getChampionIcon(matchup.enemy_champion)}" alt="${matchup.enemy_champion}">
        </div>
        <div class="matchup-info">
          <h3>${matchup.my_champion} vs ${matchup.enemy_champion}</h3>
          <span class="role-badge">${matchup.role}</span>
        </div>
        <div class="matchup-tags">
          ${tags.map(tag => `<span class="tag ${tag}">${tag}</span>`).join('')}
        </div>
      </div>
    `;
  }).join('');

  elements.matchupsList.querySelectorAll('.matchup-card').forEach(card => {
    card.addEventListener('click', () => openMatchupDetail(card.dataset.id));
  });
}

function renderHistory() {
  if (state.matches.length === 0) {
    elements.historyList.innerHTML = `
      <div class="empty-state">
        <h3>No matches</h3>
        <p>Connect to LoL client to import your match history</p>
      </div>
    `;
    return;
  }

  elements.historyList.innerHTML = state.matches.map(match => `
    <div class="match-card ${match.result}" data-id="${match.id}">
      <div class="matchup-champions">
        <img class="champion-icon" src="${getChampionIcon(match.my_champion)}" alt="${match.my_champion}">
        <span class="vs">vs</span>
        <img class="champion-icon" src="${getChampionIcon(match.enemy_champion)}" alt="${match.enemy_champion}">
      </div>
      <div class="matchup-info">
        <h3>${match.my_champion} vs ${match.enemy_champion}</h3>
        <span class="role-badge">${match.role}</span>
      </div>
      <span class="match-result ${match.result}">${match.result === 'win' ? 'Victory' : 'Defeat'}</span>
      <span class="match-date">${formatDate(match.date)}</span>
    </div>
  `).join('');

  elements.historyList.querySelectorAll('.match-card').forEach(card => {
    card.addEventListener('click', () => openMatchDetail(card.dataset.id));
  });
}

// ==================== Matchup Modal ====================

async function openMatchupDetail(id) {
  try {
    state.currentMatchup = await invoke('get_matchup', { id });
    const matchup = state.currentMatchup;
    const currentVersion = matchup.versions[matchup.current_version - 1] || matchup.versions[0];

    elements.detailMyChampionIcon.src = getChampionIcon(matchup.my_champion);
    elements.detailEnemyChampionIcon.src = getChampionIcon(matchup.enemy_champion);
    elements.detailTitle.textContent = `${matchup.my_champion} vs ${matchup.enemy_champion}`;
    elements.detailRole.textContent = matchup.role;

    renderTags(currentVersion?.tags || []);

    // Set build info as input values
    elements.detailRunes.value = currentVersion?.runes?.join(', ') || '';
    elements.detailSummoners.value = currentVersion?.summoner_spells?.join(', ') || '';
    elements.detailItems.value = currentVersion?.items?.join(', ') || '';

    elements.detailNotes.value = currentVersion?.notes || '';

    elements.detailVersion.innerHTML = matchup.versions.map((v, i) =>
      `<option value="${i + 1}">v${i + 1}</option>`
    ).join('');
    elements.detailVersion.value = matchup.current_version;
    elements.detailVersionDate.textContent = formatDate(currentVersion?.date);

    elements.modalMatchup.classList.remove('hidden');
  } catch (error) {
    console.error('Error loading matchup:', error);
  }
}

function closeMatchupModal() {
  elements.modalMatchup.classList.add('hidden');
  state.currentMatchup = null;
}

function renderTags(tags) {
  elements.detailTags.innerHTML = tags.map(tag =>
    `<span class="tag ${tag} removable" data-tag="${tag}">${tag}</span>`
  ).join('');

  elements.detailTags.querySelectorAll('.tag').forEach(tagEl => {
    tagEl.addEventListener('click', () => removeTag(tagEl.dataset.tag));
  });
}

function handleAddTag(e) {
  if (e.key === 'Enter' && e.target.value.trim()) {
    const tag = e.target.value.trim().toLowerCase().replace(/\s+/g, '-');
    const currentVersion = state.currentMatchup.versions[state.currentMatchup.current_version - 1];
    if (!currentVersion.tags) currentVersion.tags = [];
    if (!currentVersion.tags.includes(tag)) {
      currentVersion.tags.push(tag);
      renderTags(currentVersion.tags);
    }
    e.target.value = '';
  }
}

function removeTag(tag) {
  const currentVersion = state.currentMatchup.versions[state.currentMatchup.current_version - 1];
  currentVersion.tags = currentVersion.tags.filter(t => t !== tag);
  renderTags(currentVersion.tags);
}

function handleVersionChange() {
  const versionNum = parseInt(elements.detailVersion.value);
  const version = state.currentMatchup.versions[versionNum - 1];

  elements.detailNotes.value = version?.notes || '';
  elements.detailRunes.value = version?.runes?.join(', ') || '';
  elements.detailSummoners.value = version?.summoner_spells?.join(', ') || '';
  elements.detailItems.value = version?.items?.join(', ') || '';
  elements.detailVersionDate.textContent = formatDate(version?.date);
  renderTags(version?.tags || []);
}

// Helper to parse comma-separated input into array
function parseCommaSeparated(value) {
  if (!value || !value.trim()) return [];
  return value.split(',').map(s => s.trim()).filter(s => s.length > 0);
}

async function saveMatchup() {
  try {
    const currentVersion = state.currentMatchup.versions[state.currentMatchup.current_version - 1];

    const newNotes = elements.detailNotes.value;
    const newRunes = parseCommaSeparated(elements.detailRunes.value);
    const newSummoners = parseCommaSeparated(elements.detailSummoners.value);
    const newItems = parseCommaSeparated(elements.detailItems.value);
    const newTags = currentVersion.tags || [];

    // Check if anything changed
    const notesChanged = newNotes !== (currentVersion.notes || '');
    const runesChanged = JSON.stringify(newRunes) !== JSON.stringify(currentVersion.runes || []);
    const summonersChanged = JSON.stringify(newSummoners) !== JSON.stringify(currentVersion.summoner_spells || []);
    const itemsChanged = JSON.stringify(newItems) !== JSON.stringify(currentVersion.items || []);

    if (notesChanged || runesChanged || summonersChanged || itemsChanged) {
      const update = {
        notes: newNotes,
        tags: newTags,
        runes: newRunes,
        summoner_spells: newSummoners,
        items: newItems
      };

      await invoke('update_matchup', {
        id: state.currentMatchup.id,
        update
      });
    }

    closeMatchupModal();
    await loadMatchups();
    renderMatchups();
    populateChampionFilters();
  } catch (error) {
    console.error('Error saving matchup:', error);
  }
}

async function deleteMatchup() {
  if (!confirm('Are you sure you want to delete this matchup?')) return;

  try {
    await invoke('delete_matchup', { id: state.currentMatchup.id });
    closeMatchupModal();
    await loadMatchups();
    renderMatchups();
    populateChampionFilters();
  } catch (error) {
    console.error('Error deleting matchup:', error);
  }
}

// ==================== New Matchup Modal ====================

function openNewMatchupModal() {
  elements.newMyChampion.value = '';
  elements.newEnemyChampion.value = '';
  elements.newRole.value = 'top';
  elements.modalNewMatchup.classList.remove('hidden');
  elements.newMyChampion.focus();
}

function closeNewMatchupModal() {
  elements.modalNewMatchup.classList.add('hidden');
}

async function createMatchup() {
  const myChampion = elements.newMyChampion.value.trim();
  const enemyChampion = elements.newEnemyChampion.value.trim();
  const role = elements.newRole.value;

  if (!myChampion || !enemyChampion) {
    alert('Please select both champions');
    return;
  }

  try {
    const newMatchup = {
      my_champion: myChampion,
      enemy_champion: enemyChampion,
      role
    };

    await invoke('create_matchup', { matchup: newMatchup });
    closeNewMatchupModal();
    await loadMatchups();
    renderMatchups();
    populateChampionFilters();
  } catch (error) {
    console.error('Error creating matchup:', error);
  }
}

// ==================== Match Modal ====================

async function openMatchDetail(id) {
  const match = state.matches.find(m => m.id === id);
  if (!match) return;

  state.currentMatch = match;

  elements.matchMyChampionIcon.src = getChampionIcon(match.my_champion);
  elements.matchEnemyChampionIcon.src = getChampionIcon(match.enemy_champion);
  elements.matchResult.textContent = match.result === 'win' ? 'Victory' : 'Defeat';
  elements.matchResult.className = `match-result ${match.result}`;
  elements.matchDate.textContent = formatDate(match.date);
  elements.matchNotes.value = match.notes || '';

  elements.matchLinkedMatchup.innerHTML = '<option value="">Not linked</option>' +
    state.matchups.map(m =>
      `<option value="${m.id}" ${m.id === match.linked_matchup ? 'selected' : ''}>
        ${m.my_champion} vs ${m.enemy_champion}
      </option>`
    ).join('');

  elements.modalMatch.classList.remove('hidden');
}

function closeMatchModal() {
  elements.modalMatch.classList.add('hidden');
  state.currentMatch = null;
}

async function saveMatch() {
  try {
    const update = {
      notes: elements.matchNotes.value,
      linked_matchup: elements.matchLinkedMatchup.value || null
    };

    await invoke('update_match', {
      id: state.currentMatch.id,
      update
    });

    closeMatchModal();
    await loadMatches();
    renderHistory();
  } catch (error) {
    console.error('Error saving match:', error);
  }
}

// ==================== LCU Connection ====================

async function connectToLcu() {
  try {
    const result = await invoke('connect_lcu', {});
    if (result.connected) {
      state.lcuConnected = true;
      updateLcuStatus(true, result.summoner_name);
      // Import matches after connecting
      await importMatches();
    }
  } catch (error) {
    console.error('Error connecting to LCU:', error);
    updateLcuStatus(false);
    alert('Could not connect to League client. Make sure the client is running.');
  }
}

async function importMatches() {
  try {
    const imported = await invoke('import_matches', { count: 20 });
    if (imported.length > 0) {
      await loadMatches();
      renderHistory();
      console.log(`Imported ${imported.length} matches`);
    }
  } catch (error) {
    console.error('Error importing matches:', error);
  }
}

function updateLcuStatus(connected, summonerName = null) {
  const indicator = elements.lcuStatus.querySelector('.status-indicator');
  const text = elements.lcuStatus.querySelector('span:not(.status-indicator)');

  indicator.classList.toggle('connected', connected);
  text.textContent = connected
    ? `Connected: ${summonerName || 'Unknown'}`
    : 'LoL client not detected';
}

// ==================== Window Controls ====================

async function toggleAlwaysOnTop() {
  try {
    state.alwaysOnTop = !state.alwaysOnTop;
    const window = getCurrentWindow();
    await window.setAlwaysOnTop(state.alwaysOnTop);
    elements.btnAlwaysOnTop.classList.toggle('active', state.alwaysOnTop);
  } catch (error) {
    console.error('Error toggling always on top:', error);
  }
}

function handleGlobalShortcuts(e) {
  if (e.ctrlKey && e.shiftKey && e.key === 'M') {
    e.preventDefault();
    elements.searchInput.focus();
  }

  if (e.key === 'Escape') {
    closeMatchupModal();
    closeNewMatchupModal();
    closeMatchModal();
  }
}

// ==================== Utilities ====================

function getChampionIcon(championName) {
  return `${DDRAGON_BASE}/img/champion/${championName}.png`;
}

function formatDate(dateStr) {
  if (!dateStr) return '';
  const date = new Date(dateStr);
  return date.toLocaleDateString('en-US', {
    day: '2-digit',
    month: '2-digit',
    year: 'numeric',
    hour: '2-digit',
    minute: '2-digit'
  });
}

function debounce(fn, delay) {
  let timeoutId;
  return function(...args) {
    clearTimeout(timeoutId);
    timeoutId = setTimeout(() => fn.apply(this, args), delay);
  };
}

// ==================== Start Application ====================

document.addEventListener('DOMContentLoaded', init);
