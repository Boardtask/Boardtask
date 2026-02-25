// Main JavaScript module for Boardtask

const NODE_TYPES = {
    TASK: "01JNODETYPE00000000TASK000",
    BUG: "01JNODETYPE00000000BUG0000",
    EPIC: "01JNODETYPE00000000EPIC000",
    MILESTONE: "01JNODETYPE00000000MILESTON",
    SPIKE: "01JNODETYPE00000000SPIKE00",
    STORY: "01JNODETYPE00000000STORY00"
};

const DEFAULTS = {
    NODE_TYPE: NODE_TYPES.TASK,
    NODE_TITLE: "New Node",
    STATUS_ID: "01JSTATUS00000000TODO0000"
};

const TODO_STATUS_ID = '01JSTATUS00000000TODO0000';
const IN_PROGRESS_STATUS_ID = '01JSTATUS00000000INPROG00';
const DONE_STATUS_ID = '01JSTATUS00000000DONE0000';

const SEMANTIC_COLORS = {
    epic: '#9B6BCA',
    task: '#5A8FF0',
    bug: '#D65D5D',
    success: '#6BAF92',
    warning: '#D9A441'
};

function nodeTypeSlug(nodeTypeName) {
    const n = (nodeTypeName || '').toLowerCase();
    if (n === 'epic') return 'epic';
    if (n === 'bug') return 'bug';
    return 'task';
}

function minutesToAmountAndUnit(minutes) {
    if (minutes == null || minutes <= 0) return { amount: '', unit: 'minutes' };
    if (minutes >= 60 && minutes % 60 === 0) return { amount: minutes / 60, unit: 'hours' };
    return { amount: minutes, unit: 'minutes' };
}

function formatEstimatedMinutes(minutes) {
    const { amount, unit } = minutesToAmountAndUnit(minutes);
    if (amount === '') return '';
    return unit === 'hours' ? `${amount}h` : `${amount}m`;
}

/** Convert hex color to rgba string with given alpha (0–1). Handles #RGB and #RRGGBB. */
function hexToRgba(hex, alpha) {
    const m = String(hex).replace(/^#/, '').match(/^([0-9a-f]{3}|[0-9a-f]{6})$/i);
    if (!m) return hex;
    let r, g, b;
    if (m[1].length === 3) {
        r = parseInt(m[1][0] + m[1][0], 16);
        g = parseInt(m[1][1] + m[1][1], 16);
        b = parseInt(m[1][2] + m[1][2], 16);
    } else {
        r = parseInt(m[1].slice(0, 2), 16);
        g = parseInt(m[1].slice(2, 4), 16);
        b = parseInt(m[1].slice(4, 6), 16);
    }
    return `rgba(${r},${g},${b},${alpha})`;
}

/** Escape for safe use in HTML content and attribute values (prevents XSS). */
function escapeHtml(str) {
    if (str == null) return '';
    const s = String(str);
    return s
        .replace(/&/g, '&amp;')
        .replace(/</g, '&lt;')
        .replace(/>/g, '&gt;')
        .replace(/"/g, '&quot;')
        .replace(/'/g, '&#39;');
}

/**
 * Build cytoscape node label HTML with all API/DB-derived values escaped.
 * @param {object} data - Node data (label, node_type_name, node_type_color, status_name, slot_name, estimated_minutes, muted)
 * @param {{ selected: boolean, muted: boolean, filteredOut: boolean }} opts
 */
function buildNodeLabelHtml(data, opts) {
    // Group nodes: when empty show a named box; when they have children only the :parent border is shown (no card).
    const isGroup = data.isGroup === true;
    if (isGroup) {
        if (data.groupEmpty !== false) {
            const label = escapeHtml(data.label ?? 'New group');
            const selectedClass = opts.selected ? ' cy-node--selected' : '';
            const filteredClass = opts.filteredOut ? ' cy-node--filtered' : '';
            return `<div class="cy-node cy-node--compact cy-node--task${selectedClass}${filteredClass}" style="border-color: #E5E1DA; border-left-color: #E5E1DA;">
                                <div class="cy-node__header"><span class="cy-node__label" style="font-weight: 600;">${label}</span></div>
                                <div class="cy-node__meta"><span class="cy-node__type text-xs" style="color: #888888;">Group</span></div>
                            </div>`;
        }
        return '';
    }
    const typeName = escapeHtml(data.node_type_name || 'Task');
    const typeSlug = nodeTypeSlug(data.node_type_name);
    const typeColor = SEMANTIC_COLORS[typeSlug] || SEMANTIC_COLORS.task;
    const statusName = escapeHtml(data.status_name || '');
    const isDone = (data.status_id || '') === DONE_STATUS_ID;
    const isBlocked = (data.status_name || '').toLowerCase() === 'blocked';
    const checkmarkSvg = '<svg class="cy-node__status-check" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2.5" d="M5 13l4 4L19 7"/></svg>';
    const statusHtml = isDone
        ? `<div class="cy-node__status cy-node__status--done">${checkmarkSvg}<span>Done</span></div>`
        : (statusName ? `<div class="cy-node__status">${statusName}</div>` : '');
    const slotName = escapeHtml(data.slot_name || '');
    const slotHtml = slotName ? `<div class="cy-node__slot" title="${slotName}">${slotName}</div>` : '';
    const estimateStrRaw = formatEstimatedMinutes(data.estimated_minutes);
    const estimateStr = escapeHtml(estimateStrRaw);
    const estimateHtml = estimateStr ? `<div class="cy-node__estimate">${estimateStr}</div>` : '';
    const typeClass = ' cy-node--' + typeSlug;
    const warningClass = isBlocked ? ' cy-node--warning' : '';
    const compactClass = (!estimateStrRaw && !data.status_name && !data.slot_name && !isDone) ? ' cy-node--compact' : '';
    const mutedClass = (opts.muted) ? ' cy-node--muted' : '';
    const filteredClass = (opts.filteredOut) ? ' cy-node--filtered' : '';
    const doneClass = isDone ? ' cy-node--done' : '';
    const selectedClass = opts.selected ? ' cy-node--selected' : '';
    const label = escapeHtml(data.label ?? '');
    const borderColor = isDone ? hexToRgba(typeColor, 0.4) : typeColor;
    return `<div class="cy-node${typeClass}${warningClass}${compactClass}${mutedClass}${filteredClass}${doneClass}${selectedClass}" style="border-color: ${borderColor}; border-left-color: ${borderColor};">
                                <div class="cy-node__content">
                                    <div class="cy-node__header">
                                        <span class="cy-node__type" style="color: ${typeColor};">${typeName}</span>
                                        ${slotHtml}
                                    </div>
                                    <div class="cy-node__label${isDone ? ' cy-node__label--done' : ''}">${label}</div>
                                    <div class="cy-node__meta">${statusHtml}${estimateHtml}</div>
                                </div>
                            </div>`;
}

/**
 * Returns a sort function for the Dagre layout so that root nodes stay in a stable order
 * (by created_at) and do not reshuffle when adding a child under one epic. Dagre uses this
 * order as the tie-breaker when defining topology; we assign each node a rootIndex from its
 * root ancestor so the layout keeps roots fixed and only expands the relevant subtree.
 * @param {object} cy - Cytoscape instance
 * @returns {function} Comparator (a, b) for layout sort option
 */
function buildStableRootSort(cy) {
    const nodes = cy.nodes();
    if (nodes.length > 0) {
        const roots = nodes.filter(n => n.incomers().length === 0);
        const sortedRoots = roots.sort((a, b) => {
            const ta = a.data('created_at') ?? 0;
            const tb = b.data('created_at') ?? 0;
            if (ta !== tb) return ta - tb;
            return String(a.id()).localeCompare(String(b.id()));
        });
        const rootIdToIndex = {};
        sortedRoots.forEach((r, i) => { rootIdToIndex[r.id()] = i; });

        function getRootAncestorId(node, visited) {
            if (visited.has(node.id())) return node.id();
            visited.add(node.id());
            const inEdges = node.incomers();
            if (inEdges.length === 0) return node.id();
            const parent = inEdges.first().source();
            return getRootAncestorId(parent, visited);
        }

        nodes.forEach(node => {
            const rootId = getRootAncestorId(node, new Set());
            const idx = rootIdToIndex[rootId];
            node.data('rootIndex', typeof idx === 'number' ? idx : 0);
        });
    }

    return (a, b) => {
        const ri = (el) => (el.isNode() ? (el.data('rootIndex') ?? 0) : 0);
        const id = (el) => (el.isNode() ? el.id() : el.source().id() + '\0' + el.target().id());
        if (a.isNode() && b.isNode()) {
            const d = ri(a) - ri(b);
            if (d !== 0) return d;
            return String(a.id()).localeCompare(String(b.id()));
        }
        if (a.isEdge() && b.isEdge()) {
            const ra = ri(a.source()) - ri(b.source());
            if (ra !== 0) return ra;
            const rb = ri(a.target()) - ri(b.target());
            if (rb !== 0) return rb;
            return String(id(a)).localeCompare(String(id(b)));
        }
        return a.isNode() ? -1 : 1;
    };
}

const registerGraph = () => {
    if (!window.Alpine) return;

    Alpine.store('projectFlash', { show: false, message: '' });

    if (Alpine.data('graph')) return;
    if (Alpine.data('projectList')) return;

    Alpine.data('projectList', (projectId) => ({
        projectId,
        tasksData: [],
        nodeTypes: [],
        taskStatuses: [],
        projectSlots: [],
        editingNode: null,
        editingNodeOriginal: null,
        saving: false,

        async init() {
            const el = document.getElementById('project-tasks-data');
            if (el && el.textContent) {
                try {
                    this.tasksData = JSON.parse(el.textContent);
                } catch (_) {
                    this.tasksData = [];
                }
            }
            await Promise.all([
                this.fetchNodeTypes(),
                this.fetchTaskStatuses(),
                this.fetchProjectSlots()
            ]);
        },

        async fetchNodeTypes() {
            try {
                const response = await fetch('/api/node-types');
                if (!response.ok) return;
                const data = await response.json();
                this.nodeTypes = data.node_types || [];
            } catch (_) {}
        },

        async fetchTaskStatuses() {
            try {
                const response = await fetch('/api/task-statuses');
                if (!response.ok) return;
                const data = await response.json();
                this.taskStatuses = data.task_statuses || [];
            } catch (_) {}
        },

        async fetchProjectSlots() {
            try {
                const response = await fetch(`/api/projects/${this.projectId}/slots`);
                if (!response.ok) return;
                const data = await response.json();
                this.projectSlots = data.slots || [];
            } catch (_) {}
        },

        openEdit(nodeId) {
            const node = this.tasksData.find(n => n.id === nodeId);
            if (!node) return;
            const { amount, unit } = minutesToAmountAndUnit(node.estimated_minutes);
            this.editingNode = {
                id: node.id,
                title: node.title || '',
                description: node.description ?? '',
                node_type_id: (node.node_type_id != null && node.node_type_id !== '') ? node.node_type_id : DEFAULTS.NODE_TYPE,
                status_id: node.status_id ?? DEFAULTS.STATUS_ID,
                slot_id: node.slot_id ?? '',
                estimated_amount: amount,
                estimated_unit: unit
            };
            this.editingNodeOriginal = {
                title: String(this.editingNode.title ?? ''),
                description: String(this.editingNode.description ?? ''),
                node_type_id: String(this.editingNode.node_type_id ?? ''),
                status_id: String(this.editingNode.status_id ?? ''),
                slot_id: String(this.editingNode.slot_id ?? ''),
                estimated_amount: this.editingNode.estimated_amount == null || this.editingNode.estimated_amount === '' ? '' : String(this.editingNode.estimated_amount),
                estimated_unit: String(this.editingNode.estimated_unit || 'minutes')
            };
        },

        hasEditChanges() {
            if (!this.editingNode || !this.editingNodeOriginal) return false;
            const n = this.editingNode;
            const o = this.editingNodeOriginal;
            const norm = (a) => (a == null || a === '' ? '' : String(a));
            const eq = (a, b) => norm(a) === norm(b);
            return !eq(n.title, o.title) || !eq(n.description, o.description) ||
                !eq(n.node_type_id, o.node_type_id) || !eq(n.status_id, o.status_id) ||
                !eq(n.slot_id, o.slot_id) || !eq(n.estimated_amount, o.estimated_amount) ||
                !eq(n.estimated_unit, o.estimated_unit);
        },

        requestCloseEditPanel() {
            if (!this.hasEditChanges()) {
                this.closeEditPanel();
                return;
            }
            if (confirm('You have unsaved changes. Save before closing?')) {
                this.saveNode();
            } else {
                this.closeEditPanel();
            }
        },

        closeEditPanel() {
            this.editingNode = null;
            this.editingNodeOriginal = null;
        },

        async api(url, method, body = null) {
            const options = {
                method,
                headers: { 'Content-Type': 'application/json' }
            };
            if (body) options.body = JSON.stringify(body);
            const response = await fetch(url, options);
            if (!response.ok) {
                const err = await response.text();
                throw new Error(err || 'API request failed');
            }
            return response.status !== 204 ? await response.json() : null;
        },

        async saveNode() {
            if (!this.editingNode || this.saving) return;
            this.saving = true;
            const amount = this.editingNode.estimated_amount;
            const unit = this.editingNode.estimated_unit || 'minutes';
            let estimatedMinutes = null;
            if (amount !== '' && amount != null && !Number.isNaN(Number(amount))) {
                const n = Number(amount);
                estimatedMinutes = unit === 'hours' ? Math.round(n * 60) : Math.round(n);
            }
            const slotIdForApi = (this.editingNode.slot_id != null && this.editingNode.slot_id !== '') ? this.editingNode.slot_id : null;
            try {
                await this.api(`/api/projects/${this.projectId}/nodes/${this.editingNode.id}`, 'PATCH', {
                    title: this.editingNode.title,
                    description: this.editingNode.description,
                    node_type_id: this.editingNode.node_type_id,
                    status_id: this.editingNode.status_id,
                    slot_id: slotIdForApi,
                    estimated_minutes: estimatedMinutes
                });
                this.closeEditPanel();
                window.location.reload();
            } catch (err) {
                alert(err.message || 'Failed to save');
            } finally {
                this.saving = false;
            }
        }
    }));

    Alpine.data('graph', (projectId) => ({
        projectId: projectId,
        cy: null,
        selectedNodeIds: [],
        selectedEdge: null, // { sourceId, targetId } when one edge is selected
        layoutDirection: 'LR',
        nodeTypeId: DEFAULTS.NODE_TYPE,
        nodeTypes: [],
        taskStatuses: [],
        projectSlots: [],
        editingNode: null, // { id, title, description, node_type_id, status_id, slot_id, estimated_amount, estimated_unit }
        editingNodeOriginal: null,
        saving: false,
        settingsOpen: false,
        highlightBlockedTodos: true,
        progressFilter: '', // '' | 'todo' | 'in_progress' | 'done'
        editingSlotId: null,
        editingSlotName: '',
        newSlotName: '',
        slotError: '',
        groupListVersion: 0,
        toolbarMenu: null, // 'add' | 'filter' | 'group' | null

        setToolbarMenu(menu) {
            this.toolbarMenu = menu;
        },
        toggleToolbarMenu(menu) {
            this.toolbarMenu = this.toolbarMenu === menu ? null : menu;
        },

        isGroupNode(nodeOrId) {
            if (!this.cy) return false;
            const node = typeof nodeOrId === 'string' ? this.cy.$id(nodeOrId) : nodeOrId;
            return node.length !== 0 && node.data('isGroup') === true;
        },

        isTemporaryGroupNode(nodeOrId) {
            if (!this.cy) return false;
            const node = typeof nodeOrId === 'string' ? this.cy.$id(nodeOrId) : nodeOrId;
            return node.length !== 0 && node.data('isGroup') === true && node.data('isTemporary') === true;
        },

        matchesProgressFilter(statusId) {
            if (!this.progressFilter) return true;
            const sid = statusId || DEFAULTS.STATUS_ID;
            switch (this.progressFilter) {
                case 'todo': return sid === TODO_STATUS_ID;
                case 'in_progress': return sid === IN_PROGRESS_STATUS_ID;
                case 'done': return sid === DONE_STATUS_ID;
                default: return true;
            }
        },

        async init() {
            this.cy = cytoscape({
                container: this.$refs.canvas,
                boxSelectionEnabled: false,
                autounselectify: false,
                style: [
                    {
                        selector: 'node',
                        style: {
                            'shape': 'round-rectangle',
                            'width': 220,
                            'height': 80,
                            'opacity': 0,
                            'label': ''
                        }
                    },
                    {
                        selector: 'edge',
                        style: {
                            'width': 1.5,
                            'line-color': '#B8B0A6',
                            'target-arrow-color': '#B8B0A6',
                            'target-arrow-shape': 'triangle',
                            'curve-style': 'taxi',
                            'taxi-direction': 'vertical',
                            'taxi-turn': 5,
                            'taxi-turn-min-distance': 100,
                            'target-arrow-shape': 'triangle',
                            'opacity': 0.7
                        }
                    },
                    {
                        selector: 'edge:selected',
                        style: {
                            'width': 2,
                            'line-color': '#5A8FF0',
                            'target-arrow-color': '#5A8FF0',
                            'opacity': 0.9
                        }
                    },
                    {
                        selector: ':parent',
                        style: {
                            'shape': 'rectangle',
                            'background-color': 'rgba(255, 255, 255, 0.3)',
                            'border-width': 2,
                            'border-color': '#E5E1DA',
                            'padding': 20,
                            'border-opacity': 0.8,
                            'opacity': 1
                        }
                    },
                    {
                        selector: 'node[isGroup]',
                        style: {
                            'opacity': 1,
                            'background-color': '#f7f5f4'
                        }
                    }
                ],
                elements: [], // Initialized as empty, fetched via API
                layout: {
                    name: 'dagre',
                    rankDir: this.layoutDirection,
                    nodeSep: 60,
                    rankSep: 100
                }
            });

            if (this.cy.nodeHtmlLabel) {
                this.cy.nodeHtmlLabel([
                    {
                        query: 'node',
                        halign: 'center',
                        valign: 'center',
                        halignBox: 'center',
                        valignBox: 'center',
                        tpl: (data) => buildNodeLabelHtml(data, {
                            selected: false,
                            muted: this.highlightBlockedTodos && data.muted,
                            filteredOut: !!data.filteredOut
                        })
                    },
                    {
                        query: 'node:selected',
                        halign: 'center',
                        valign: 'center',
                        halignBox: 'center',
                        valignBox: 'center',
                        tpl: (data) => buildNodeLabelHtml(data, {
                            selected: true,
                            muted: this.highlightBlockedTodos && data.muted,
                            filteredOut: !!data.filteredOut
                        })
                    }
                ]);
            }

            this.cy.on('select', 'node', (evt) => {
                this.settingsOpen = false;
                this.cy.edges().unselect();
                this.selectedEdge = null;

                const node = evt.target;
                const id = node.id();

                if (this.editingNode && this.editingNode.id !== id && this.hasEditChanges()) {
                    this.requestCloseEditPanel({ thenSelectNodeId: id });
                    return;
                }

                if (!this.selectedNodeIds.includes(id)) {
                    this.selectedNodeIds.push(id);
                }

                // Group nodes: open edit panel with name-only (isGroup: true)
                if (this.isGroupNode(node)) {
                    if (this.selectedNodeIds.length > 2) {
                        const firstId = this.selectedNodeIds.shift();
                        this.cy.$id(firstId).unselect();
                    }
                    this.editingNode = {
                        id,
                        title: node.data('label') ?? 'New group',
                        isGroup: true
                    };
                    this.editingNodeOriginal = { id, title: String(this.editingNode.title ?? ''), isGroup: true };
                    this.refreshNodeLabels();
                    return;
                }

                // Set editing node when a single node is selected or is the last selected
                const nodeTypeId = node.data('node_type_id');
                const statusId = node.data('status_id');
                const slotId = node.data('slot_id');
                const { amount: estimatedAmount, unit: estimatedUnit } = minutesToAmountAndUnit(node.data('estimated_minutes'));
                this.editingNode = {
                    id: id,
                    title: node.data('label'),
                    description: node.data('description') || '',
                    node_type_id: (nodeTypeId != null && nodeTypeId !== '') ? String(nodeTypeId) : DEFAULTS.NODE_TYPE,
                    status_id: (statusId != null && statusId !== '') ? String(statusId) : DEFAULTS.STATUS_ID,
                    slot_id: (slotId != null && slotId !== '') ? String(slotId) : '',
                    estimated_amount: estimatedAmount,
                    estimated_unit: estimatedUnit
                };
                this.editingNodeOriginal = {
                    title: String(this.editingNode.title ?? ''),
                    description: String(this.editingNode.description ?? ''),
                    node_type_id: String(this.editingNode.node_type_id ?? ''),
                    status_id: String(this.editingNode.status_id ?? ''),
                    slot_id: String(this.editingNode.slot_id ?? ''),
                    estimated_amount: this.editingNode.estimated_amount == null || this.editingNode.estimated_amount === '' ? '' : String(this.editingNode.estimated_amount),
                    estimated_unit: String(this.editingNode.estimated_unit || 'minutes')
                };

                if (this.selectedNodeIds.length > 2) {
                    const firstId = this.selectedNodeIds.shift();
                    this.cy.$id(firstId).unselect();
                }
                this.refreshNodeLabels();
            });

            this.cy.on('unselect', 'node', (evt) => {
                const id = evt.target.id();
                this.selectedNodeIds = this.selectedNodeIds.filter(nodeId => nodeId !== id);
                this.refreshNodeLabels();
            });

            this.cy.on('tap', async (evt) => {
                if (evt.target === this.cy) {
                    await this.requestCloseEditPanel();
                    this.cy.nodes().unselect();
                    this.cy.edges().unselect();
                    this.selectedNodeIds = [];
                    this.selectedEdge = null;
                    this.refreshNodeLabels();
                }
            });

            this.cy.on('tap', 'edge', async (evt) => {
                await this.requestCloseEditPanel();
                const edge = evt.target;
                this.selectedEdge = { sourceId: edge.source().id(), targetId: edge.target().id() };
                this.cy.nodes().unselect();
                this.selectedNodeIds = [];
                this.refreshNodeLabels();
                this.cy.edges().unselect();
                edge.select();
            });

            this.cy.on('dbltap', 'node', (evt) => {
                const node = evt.target;
                const neighborhood = node.closedNeighborhood();
                this.cy.animate({
                    fit: { eles: neighborhood, padding: 100 },
                    duration: 300
                });
            });

            const escapeHandler = (e) => {
                if (e.key === 'Escape' && this.editingNode) this.requestCloseEditPanel();
            };
            document.addEventListener('keydown', escapeHandler);

            await Promise.all([this.fetchNodeTypes(), this.fetchTaskStatuses(), this.fetchProjectSlots()]);
            await this.fetchGraph();
        },

        async fetchGraph() {
            try {
                const response = await fetch(`/api/projects/${this.projectId}/graph`);
                if (!response.ok) throw new Error('Failed to fetch graph');
                const data = await response.json();

                const groupIds = new Set(data.nodes.map(n => n.parent_id).filter(Boolean));
                const parentIdsById = {};
                data.edges.forEach(e => {
                    if (!parentIdsById[e.child_id]) parentIdsById[e.child_id] = [];
                    parentIdsById[e.child_id].push(e.parent_id);
                });
                const statusById = {};
                data.nodes.forEach(n => { statusById[n.id] = n.status_id ?? DEFAULTS.STATUS_ID; });
                const isRoot = (id) => !data.edges.some(e => e.child_id === id);
                const isDone = (id) => (statusById[id] || DEFAULTS.STATUS_ID) === DONE_STATUS_ID;
                const hasBlockingParent = (id) => {
                    const pids = parentIdsById[id] || [];
                    return pids.some(pid => !isRoot(pid) && !isDone(pid));
                };

                const elements = [
                    ...data.nodes.map(n => {
                        const type = this.nodeTypes.find(t => t.id === n.node_type_id);
                        const status = this.taskStatuses.find(s => s.id === (n.status_id ?? DEFAULTS.STATUS_ID));
                        const slot = this.projectSlots.find(s => s.id === (n.slot_id || ''));
                        const root = isRoot(n.id);
                        const done = isDone(n.id);
                        const muted = !root && !done && hasBlockingParent(n.id);
                        const isGroupNode = groupIds.has(n.id);
                        const statusId = n.status_id ?? DEFAULTS.STATUS_ID;
                        const filteredOut = this.progressFilter ? !this.matchesProgressFilter(statusId) : false;
                        return {
                            group: 'nodes',
                            data: {
                                id: n.id,
                                parent: n.parent_id || undefined,
                                label: n.title,
                                description: n.description,
                                node_type_id: n.node_type_id,
                                node_type_name: type ? type.name : '',
                                node_type_color: type ? type.color : '#4F46E5',
                                status_id: statusId,
                                status_name: status ? status.name : 'To do',
                                slot_id: n.slot_id ?? '',
                                slot_name: slot ? slot.name : '',
                                estimated_minutes: n.estimated_minutes ?? null,
                                muted: !!muted,
                                filteredOut: !!filteredOut,
                                isGroup: isGroupNode,
                                groupEmpty: isGroupNode ? false : undefined,
                                created_at: n.created_at
                            }
                        };
                    }),
                    ...data.edges.map(e => ({ group: 'edges', data: { source: e.parent_id, target: e.child_id } }))
                ];

                this.cy.elements().remove();
                this.cy.add(elements);
                this.selectedEdge = null;
                this.runLayout({ fit: true });
            } catch (error) {
                console.error('Fetch error:', error);
                alert('Could not load graph data.');
            }
        },

        recomputeMutedForGraph() {
            const cy = this.cy;
            const parentIdsById = {};
            cy.edges().forEach(edge => {
                const childId = edge.target().id();
                const parentId = edge.source().id();
                if (!parentIdsById[childId]) parentIdsById[childId] = [];
                parentIdsById[childId].push(parentId);
            });
            const statusById = {};
            cy.nodes().forEach(node => { statusById[node.id()] = node.data('status_id') || DEFAULTS.STATUS_ID; });
            const isRoot = (id) => !cy.edges().some(e => e.target().id() === id);
            const isDone = (id) => (statusById[id] || DEFAULTS.STATUS_ID) === DONE_STATUS_ID;
            const hasBlockingParent = (id) => {
                const pids = parentIdsById[id] || [];
                return pids.some(pid => !isRoot(pid) && !isDone(pid));
            };
            cy.nodes().forEach(node => {
                const id = node.id();
                const root = isRoot(id);
                const done = isDone(id);
                const muted = !root && !done && hasBlockingParent(id);
                node.data('muted', !!muted);
            });
            try {
                const nh = cy.nodeHtmlLabel && cy.nodeHtmlLabel();
                if (nh && typeof nh.update === 'function') nh.update();
            } catch (_) {}
        },

        recomputeFilteredForGraph() {
            if (!this.cy) return;
            const statusIdFor = (node) => node.data('status_id') || DEFAULTS.STATUS_ID;
            this.cy.nodes().forEach(node => {
                const filteredOut = this.progressFilter ? !this.matchesProgressFilter(statusIdFor(node)) : false;
                node.data('filteredOut', !!filteredOut);
            });
            try {
                const nh = this.cy.nodeHtmlLabel && this.cy.nodeHtmlLabel();
                if (nh && typeof nh.update === 'function') nh.update();
            } catch (_) {}
        },

        refreshNodeLabels() {
            try {
                const cy = this.cy;
                if (cy && cy.nodeHtmlLabel) {
                    const nh = cy.nodeHtmlLabel();
                    if (nh && typeof nh.update === 'function') nh.update();
                }
            } catch (_) {}
        },

        async fetchNodeTypes() {
            try {
                const response = await fetch('/api/node-types');
                if (!response.ok) throw new Error('Failed to fetch node types');
                const data = await response.json();
                this.nodeTypes = data.node_types;
            } catch (error) {
                console.error('Fetch error:', error);
            }
        },

        async fetchTaskStatuses() {
            try {
                const response = await fetch('/api/task-statuses');
                if (!response.ok) throw new Error('Failed to fetch task statuses');
                const data = await response.json();
                this.taskStatuses = data.task_statuses;
            } catch (error) {
                console.error('Fetch error:', error);
            }
        },

        async fetchProjectSlots() {
            try {
                const response = await fetch(`/api/projects/${this.projectId}/slots`);
                if (!response.ok) throw new Error('Failed to fetch project slots');
                const data = await response.json();
                this.projectSlots = data.slots;
            } catch (error) {
                console.error('Fetch error:', error);
            }
        },

        async api(url, method, body = null) {
            const options = {
                method,
                headers: { 'Content-Type': 'application/json' }
            };
            if (body) options.body = JSON.stringify(body);

            const response = await fetch(url, options);
            if (!response.ok) {
                const error = await response.text();
                throw new Error(error || 'API request failed');
            }
            return response.status !== 204 ? await response.json() : null;
        },

        async addNode() {
            try {
                const title = DEFAULTS.NODE_TITLE;

                const node = await this.api(`/api/projects/${this.projectId}/nodes`, 'POST', {
                    node_type_id: this.nodeTypeId,
                    title: title,
                    description: ""
                });

                const type = this.nodeTypes.find(t => t.id === node.node_type_id);
                const status = this.taskStatuses.find(s => s.id === (node.status_id ?? DEFAULTS.STATUS_ID));
                const slot = this.projectSlots.find(s => s.id === (node.slot_id || ''));
                const statusId = node.status_id ?? DEFAULTS.STATUS_ID;
                const filteredOut = this.progressFilter ? !this.matchesProgressFilter(statusId) : false;
                this.cy.add({
                    group: 'nodes',
                    data: {
                        id: node.id,
                        label: node.title,
                        description: node.description,
                        node_type_id: node.node_type_id,
                        node_type_name: type ? type.name : '',
                        status_id: statusId,
                        status_name: status ? status.name : 'To do',
                        node_type_color: type ? type.color : '#4F46E5',
                        slot_id: node.slot_id ?? '',
                        slot_name: slot ? slot.name : '',
                        estimated_minutes: node.estimated_minutes ?? null,
                        muted: false,
                        filteredOut: !!filteredOut
                    }
                });
                this.runLayout({ fit: true });
            } catch (error) {
                alert(`Error adding node: ${error.message}`);
            }
        },

        createGroup() {
            const id = crypto.randomUUID();
            this.cy.add({
                group: 'nodes',
                data: {
                    id,
                    label: 'New group',
                    isGroup: true,
                    isTemporary: true,
                    groupEmpty: true,
                    node_type_name: 'Group',
                    node_type_color: '#94A3B8',
                    status_name: '',
                    slot_name: '',
                    estimated_minutes: null,
                    muted: false
                }
            });
            this.groupListVersion++;
            this.runLayout({ fit: true });
            this.cy.nodes().unselect();
            this.cy.$id(id).select();
        },

        async addSelectedToGroup(groupId) {
            if (!groupId || this.selectedNodeIds.length === 0) return;
            const taskIds = this.selectedNodeIds.filter(id => !this.isGroupNode(id));
            if (taskIds.length === 0) return;

            try {
                if (this.isTemporaryGroupNode(groupId)) {
                    const groupNode = this.cy.$id(groupId);
                    if (!groupNode.length) return;
                    const label = groupNode.data('label') || 'New group';
                    const newGroup = await this.api(`/api/projects/${this.projectId}/nodes`, 'POST', {
                        node_type_id: DEFAULTS.NODE_TYPE,
                        title: label,
                        description: '',
                        status_id: DEFAULTS.STATUS_ID
                    });
                    const newGroupId = newGroup.id;
                    for (const id of taskIds) {
                        await this.api(`/api/projects/${this.projectId}/nodes/${id}`, 'PATCH', { parent_id: newGroupId });
                    }
                    groupNode.children().move({ parent: null });
                    this.cy.remove(groupNode);
                    this.cy.add({
                        group: 'nodes',
                        data: {
                            id: newGroupId,
                            label: newGroup.title,
                            isGroup: true,
                            groupEmpty: false,
                            node_type_name: 'Group',
                            node_type_color: '#94A3B8',
                            status_name: '',
                            slot_name: '',
                            estimated_minutes: null,
                            muted: false
                        }
                    });
                    for (const id of taskIds) {
                        const node = this.cy.$id(id);
                        if (node.length) node.move({ parent: newGroupId });
                    }
                    this.groupListVersion++;
                } else {
                    for (const id of taskIds) {
                        await this.api(`/api/projects/${this.projectId}/nodes/${id}`, 'PATCH', { parent_id: groupId });
                        const node = this.cy.$id(id);
                        if (node.length) node.move({ parent: groupId });
                    }
                }
                this.refreshNodeLabels();
                this.runLayout();
            } catch (error) {
                alert(`Error adding to group: ${error.message}`);
            }
        },

        compoundNodes() {
            if (!this.cy) return [];
            const all = this.cy.nodes().filter(n => n.data('isGroup') === true).toArray();
            return all.map(n => ({ id: n.id(), label: n.data('label') || 'Group' }));
        },

        async removeSelectedFromGroup() {
            if (this.selectedNodeIds.length === 0) return;
            const parentsToCheck = new Set();
            try {
                for (const id of this.selectedNodeIds) {
                    const node = this.cy.$id(id);
                    if (node.length && node.parent().length) {
                        const parentId = node.parent().id();
                        parentsToCheck.add(parentId);
                        const isPersisted = !(node.data('isGroup') && node.data('isTemporary'));
                        if (isPersisted) {
                            await this.api(`/api/projects/${this.projectId}/nodes/${id}`, 'PATCH', { parent_id: null });
                        }
                        node.move({ parent: null });
                    }
                }
                for (const parentId of parentsToCheck) {
                    const parent = this.cy.$id(parentId);
                    if (parent.length && parent.children().length === 0) {
                        parent.data('groupEmpty', true);
                    }
                }
                this.refreshNodeLabels();
                this.runLayout();
            } catch (error) {
                alert(`Error removing from group: ${error.message}`);
            }
        },

        hasSelectedInGroup() {
            if (!this.cy || this.selectedNodeIds.length === 0) return false;
            return this.selectedNodeIds.some(id => {
                const node = this.cy.$id(id);
                return node.length && node.parent().length;
            });
        },

        async addChildNode() {
            if (this.selectedNodeIds.length === 0) return;
            const parentId = this.selectedNodeIds[this.selectedNodeIds.length - 1];
            const parentNode = this.cy.$id(parentId);
            const groupId = parentNode.length && parentNode.parent().length ? parentNode.parent().id() : null;

            try {
                const title = DEFAULTS.NODE_TITLE;

                const node = await this.api(`/api/projects/${this.projectId}/nodes`, 'POST', {
                    node_type_id: this.nodeTypeId,
                    title: title,
                    description: "",
                    ...(groupId && { parent_id: groupId })
                });

                await this.api(`/api/projects/${this.projectId}/edges`, 'POST', {
                    parent_id: parentId,
                    child_id: node.id
                });

                const type = this.nodeTypes.find(t => t.id === node.node_type_id);
                const status = this.taskStatuses.find(s => s.id === (node.status_id ?? DEFAULTS.STATUS_ID));
                const slot = this.projectSlots.find(s => s.id === (node.slot_id || ''));
                const isParentRoot = parentNode.incomers().length === 0;
                const parentDone = (parentNode.data('status_id') || DEFAULTS.STATUS_ID) === DONE_STATUS_ID;
                const newNodeDone = (node.status_id ?? DEFAULTS.STATUS_ID) === DONE_STATUS_ID;
                const muted = !newNodeDone && !isParentRoot && !parentDone;
                const statusId = node.status_id ?? DEFAULTS.STATUS_ID;
                const filteredOut = this.progressFilter ? !this.matchesProgressFilter(statusId) : false;
                this.cy.add([
                    {
                        group: 'nodes',
                        data: {
                            id: node.id,
                            parent: groupId || undefined,
                            label: node.title,
                            description: node.description,
                            node_type_id: node.node_type_id,
                            node_type_name: type ? type.name : '',
                            node_type_color: type ? type.color : '#4F46E5',
                            status_id: statusId,
                            status_name: status ? status.name : 'To do',
                            slot_id: node.slot_id ?? '',
                            slot_name: slot ? slot.name : '',
                            estimated_minutes: node.estimated_minutes ?? null,
                            muted: !!muted,
                            filteredOut: !!filteredOut,
                            created_at: node.created_at
                        }
                    },
                    { group: 'edges', data: { source: parentId, target: node.id } }
                ]);
                this.runLayout();
                this.cy.nodes().unselect();
                this.cy.$id(node.id).select();
                this.selectedNodeIds = [node.id];
                const { amount: estimatedAmount, unit: estimatedUnit } = minutesToAmountAndUnit(node.estimated_minutes ?? null);
                this.editingNode = {
                    id: node.id,
                    title: node.title,
                    description: node.description || '',
                    node_type_id: (node.node_type_id != null && node.node_type_id !== '') ? String(node.node_type_id) : DEFAULTS.NODE_TYPE,
                    status_id: (node.status_id != null && node.status_id !== '') ? String(node.status_id) : DEFAULTS.STATUS_ID,
                    slot_id: (node.slot_id != null && node.slot_id !== '') ? String(node.slot_id) : '',
                    estimated_amount: estimatedAmount,
                    estimated_unit: estimatedUnit
                };
                this.editingNodeOriginal = {
                    title: String(this.editingNode.title ?? ''),
                    description: String(this.editingNode.description ?? ''),
                    node_type_id: String(this.editingNode.node_type_id ?? ''),
                    status_id: String(this.editingNode.status_id ?? ''),
                    slot_id: String(this.editingNode.slot_id ?? ''),
                    estimated_amount: this.editingNode.estimated_amount == null || this.editingNode.estimated_amount === '' ? '' : String(this.editingNode.estimated_amount),
                    estimated_unit: String(this.editingNode.estimated_unit || 'minutes')
                };
            } catch (error) {
                alert(`Error adding child node: ${error.message}`);
            }
        },

        async addParentNode() {
            if (this.selectedNodeIds.length === 0) return;
            const childId = this.selectedNodeIds[this.selectedNodeIds.length - 1];

            try {
                const title = DEFAULTS.NODE_TITLE;

                const node = await this.api(`/api/projects/${this.projectId}/nodes`, 'POST', {
                    node_type_id: this.nodeTypeId,
                    title: title,
                    description: ""
                });

                await this.api(`/api/projects/${this.projectId}/edges`, 'POST', {
                    parent_id: node.id,
                    child_id: childId
                });

                const type = this.nodeTypes.find(t => t.id === node.node_type_id);
                const status = this.taskStatuses.find(s => s.id === (node.status_id ?? DEFAULTS.STATUS_ID));
                const slot = this.projectSlots.find(s => s.id === (node.slot_id || ''));
                const statusId = node.status_id ?? DEFAULTS.STATUS_ID;
                const filteredOut = this.progressFilter ? !this.matchesProgressFilter(statusId) : false;
                this.cy.add([
                    {
                        group: 'nodes',
                        data: {
                            id: node.id,
                            label: node.title,
                            description: node.description,
                            node_type_id: node.node_type_id,
                            node_type_name: type ? type.name : '',
                            node_type_color: type ? type.color : '#4F46E5',
                            status_id: statusId,
                            status_name: status ? status.name : 'To do',
                            slot_id: node.slot_id ?? '',
                            slot_name: slot ? slot.name : '',
                            estimated_minutes: node.estimated_minutes ?? null,
                            muted: false,
                            filteredOut: !!filteredOut,
                            created_at: node.created_at
                        }
                    },
                    { group: 'edges', data: { source: node.id, target: childId } }
                ]);
                this.runLayout();
                this.cy.nodes().unselect();
                this.cy.$id(node.id).select();
                this.selectedNodeIds = [node.id];
                const { amount: estimatedAmount, unit: estimatedUnit } = minutesToAmountAndUnit(node.estimated_minutes ?? null);
                this.editingNode = {
                    id: node.id,
                    title: node.title,
                    description: node.description || '',
                    node_type_id: (node.node_type_id != null && node.node_type_id !== '') ? String(node.node_type_id) : DEFAULTS.NODE_TYPE,
                    status_id: (node.status_id != null && node.status_id !== '') ? String(node.status_id) : DEFAULTS.STATUS_ID,
                    slot_id: (node.slot_id != null && node.slot_id !== '') ? String(node.slot_id) : '',
                    estimated_amount: estimatedAmount,
                    estimated_unit: estimatedUnit
                };
                this.editingNodeOriginal = {
                    title: String(this.editingNode.title ?? ''),
                    description: String(this.editingNode.description ?? ''),
                    node_type_id: String(this.editingNode.node_type_id ?? ''),
                    status_id: String(this.editingNode.status_id ?? ''),
                    slot_id: String(this.editingNode.slot_id ?? ''),
                    estimated_amount: this.editingNode.estimated_amount == null || this.editingNode.estimated_amount === '' ? '' : String(this.editingNode.estimated_amount),
                    estimated_unit: String(this.editingNode.estimated_unit || 'minutes')
                };
            } catch (error) {
                alert(`Error adding parent node: ${error.message}`);
            }
        },

        async connectNodes() {
            if (this.selectedNodeIds.length !== 2) return;

            const sourceId = this.selectedNodeIds[0];
            const targetId = this.selectedNodeIds[1];
            const sourceIsGroup = this.isGroupNode(sourceId);
            const targetIsGroup = this.isGroupNode(targetId);

            try {
                await this.api(`/api/projects/${this.projectId}/edges`, 'POST', {
                    parent_id: sourceId,
                    child_id: targetId
                });

                this.cy.add({ group: 'edges', data: { source: sourceId, target: targetId } });
                this.runLayout();
                if (!sourceIsGroup && !targetIsGroup) {
                    this.recomputeMutedForGraph();
                }
            } catch (error) {
                alert(`Error connecting nodes: ${error.message}`);
            }
        },

        async disconnectNodes() {
            if (this.selectedEdge) {
                const { sourceId: n1, targetId: n2 } = this.selectedEdge;
                const n1IsGroup = this.isGroupNode(n1);
                const n2IsGroup = this.isGroupNode(n2);
                try {
                    await this.api(`/api/projects/${this.projectId}/edges`, 'DELETE', {
                        parent_id: n1,
                        child_id: n2
                    });
                    this.cy.edges().filter(e => e.source().id() === n1 && e.target().id() === n2).remove();
                    this.selectedEdge = null;
                    this.runLayout();
                    if (!n1IsGroup && !n2IsGroup) {
                        this.recomputeMutedForGraph();
                    }
                } catch (error) {
                    alert(`Error disconnecting nodes: ${error.message}`);
                }
                return;
            }
            if (this.selectedNodeIds.length !== 2) return;

            const n1 = this.selectedNodeIds[0];
            const n2 = this.selectedNodeIds[1];
            const n1IsGroup = this.isGroupNode(n1);
            const n2IsGroup = this.isGroupNode(n2);

            try {
                await this.api(`/api/projects/${this.projectId}/edges`, 'DELETE', {
                    parent_id: n1,
                    child_id: n2
                });

                this.cy.edges().filter(e => e.source().id() === n1 && e.target().id() === n2).remove();
                this.runLayout();
                if (!n1IsGroup && !n2IsGroup) {
                    this.recomputeMutedForGraph();
                }
            } catch (error) {
                alert(`Error disconnecting nodes: ${error.message}`);
            }
        },

        toggleDirection() {
            this.layoutDirection = this.layoutDirection === 'TB' ? 'LR' : 'TB';
            this.runLayout();
        },

        async removeNode() {
            const nodes = this.cy.nodes(':selected');
            if (nodes.length === 0) return;

            try {
                for (const node of nodes) {
                    const id = node.id();
                    if (this.isTemporaryGroupNode(node)) {
                        node.children().move({ parent: null });
                        this.cy.remove(node);
                        this.groupListVersion++;
                        continue;
                    }
                    if (node.data('isGroup') === true) {
                        await this.api(`/api/projects/${this.projectId}/nodes/${id}`, 'DELETE');
                        node.children().move({ parent: null });
                        this.cy.remove(node);
                        this.groupListVersion++;
                        continue;
                    }
                    await this.api(`/api/projects/${this.projectId}/nodes/${id}`, 'DELETE');
                    this.cy.remove(node);
                }
                this.selectedNodeIds = [];
                this.editingNode = null;
                this.editingNodeOriginal = null;
                this.runLayout();
            } catch (error) {
                alert(`Error removing node: ${error.message}`);
            }
        },

        async saveNode() {
            if (!this.editingNode || this.saving) return;

            if (this.editingNode.isGroup) {
                const id = this.editingNode.id;
                const node = this.cy.$id(id);
                if (node.length && node.data('isTemporary') === true) {
                    this.cy.$id(id).data('label', this.editingNode.title);
                    this.groupListVersion++;
                } else {
                    await this.api(`/api/projects/${this.projectId}/nodes/${id}`, 'PATCH', { title: this.editingNode.title });
                    this.cy.$id(id).data('label', this.editingNode.title);
                }
                this.refreshNodeLabels();
                this.editingNode = null;
                return;
            }

            this.saving = true;

            const amount = this.editingNode.estimated_amount;
            const unit = this.editingNode.estimated_unit || 'minutes';
            let estimatedMinutes = null;
            if (amount !== '' && amount != null && !Number.isNaN(Number(amount))) {
                const n = Number(amount);
                estimatedMinutes = unit === 'hours' ? Math.round(n * 60) : Math.round(n);
            }

            const slotIdForApi = (this.editingNode.slot_id != null && this.editingNode.slot_id !== '') ? this.editingNode.slot_id : null;
            try {
                await this.api(`/api/projects/${this.projectId}/nodes/${this.editingNode.id}`, 'PATCH', {
                    title: this.editingNode.title,
                    description: this.editingNode.description,
                    node_type_id: this.editingNode.node_type_id,
                    status_id: this.editingNode.status_id,
                    slot_id: slotIdForApi,
                    estimated_minutes: estimatedMinutes
                });

                // Update Cytoscape node
                const type = this.nodeTypes.find(t => t.id === this.editingNode.node_type_id);
                const status = this.taskStatuses.find(s => s.id === this.editingNode.status_id);
                const slot = this.projectSlots.find(s => s.id === (this.editingNode.slot_id || ''));
                const cyNode = this.cy.$id(this.editingNode.id);
                cyNode.data('label', this.editingNode.title);
                cyNode.data('description', this.editingNode.description);
                cyNode.data('node_type_id', this.editingNode.node_type_id);
                cyNode.data('node_type_name', type ? type.name : '');
                cyNode.data('node_type_color', type ? type.color : '#4F46E5');
                cyNode.data('status_id', this.editingNode.status_id);
                cyNode.data('status_name', status ? status.name : 'To do');
                cyNode.data('slot_id', this.editingNode.slot_id || '');
                cyNode.data('slot_name', slot ? slot.name : '');
                cyNode.data('estimated_minutes', estimatedMinutes);

                this.recomputeMutedForGraph();
                this.recomputeFilteredForGraph();
                Alpine.store('projectFlash', { show: true, message: 'Save Success!' });
                setTimeout(() => {
                    Alpine.store('projectFlash', { show: false, message: '' });
                }, 3000);

                // Sync original so we don't prompt again when closing
                const n = this.editingNode;
                this.editingNodeOriginal = {
                    title: String(n.title ?? ''),
                    description: String(n.description ?? ''),
                    node_type_id: String(n.node_type_id ?? ''),
                    status_id: String(n.status_id ?? ''),
                    slot_id: String(n.slot_id ?? ''),
                    estimated_amount: n.estimated_amount == null || n.estimated_amount === '' ? '' : String(n.estimated_amount),
                    estimated_unit: String(n.estimated_unit || 'minutes')
                };
            } catch (error) {
                alert(`Error saving node: ${error.message}`);
            } finally {
                this.saving = false;
            }
        },

        hasEditChanges() {
            if (!this.editingNode || !this.editingNodeOriginal) return false;
            if (this.editingNode.isGroup) {
                const o = this.editingNodeOriginal;
                return String(this.editingNode.title ?? '') !== String(o.title ?? '');
            }
            const n = this.editingNode;
            const o = this.editingNodeOriginal;
            const norm = (a) => (a == null || a === '' ? '' : String(a));
            const eq = (a, b) => norm(a) === norm(b);
            return !eq(n.title, o.title) || !eq(n.description, o.description) ||
                !eq(n.node_type_id, o.node_type_id) || !eq(n.status_id, o.status_id) ||
                !eq(n.slot_id, o.slot_id) || !eq(n.estimated_amount, o.estimated_amount) ||
                !eq(n.estimated_unit, o.estimated_unit);
        },

        doCloseEditPanel(options = {}) {
            const id = options.editingNodeId ?? this.editingNode?.id;
            if (id && this.cy) {
                this.cy.$id(id).unselect();
            }
            this.editingNode = null;
            this.editingNodeOriginal = null;
            if (options.thenSelectNodeId && this.cy) {
                this.cy.$id(options.thenSelectNodeId).select();
            }
        },

        async requestCloseEditPanel(options = {}) {
            const opts = { ...options, editingNodeId: this.editingNode?.id };
            if (!this.hasEditChanges()) {
                this.doCloseEditPanel(opts);
                return;
            }
            if (confirm('You have unsaved changes. Save before closing?')) {
                await this.saveNode();
            }
            this.doCloseEditPanel(opts);
        },

        closeEditPanel() {
            this.requestCloseEditPanel();
        },

        onEscape() {
            if (this.editingNode) this.requestCloseEditPanel();
        },

        openSettings() {
            this.settingsOpen = true;
            this.editingNode = null;
            this.editingNodeOriginal = null;
            this.selectedNodeIds = [];
            if (this.cy) {
                this.cy.nodes().unselect();
            }
            this.editingSlotId = null;
            this.editingSlotName = '';
            this.newSlotName = '';
            this.slotError = '';
            this.fetchProjectSlots();
        },

        closeSettings() {
            this.settingsOpen = false;
            this.editingSlotId = null;
            this.editingSlotName = '';
            this.newSlotName = '';
            this.slotError = '';
        },

        startEditSlot(slot) {
            this.editingSlotId = slot.id;
            this.editingSlotName = slot.name;
        },

        cancelEditSlot() {
            this.editingSlotId = null;
            this.editingSlotName = '';
        },

        async saveEditSlot() {
            if (this.editingSlotId == null) return;
            const name = (this.editingSlotName || '').trim();
            if (!name) {
                this.slotError = 'Name is required';
                return;
            }
            this.slotError = '';
            try {
                await this.updateSlot(this.editingSlotId, name);
                this.editingSlotId = null;
                this.editingSlotName = '';
            } catch (e) {
                this.slotError = e.message || 'Failed to update slot';
            }
        },

        async addSlot() {
            const name = (this.newSlotName || '').trim();
            if (!name) {
                this.slotError = 'Name is required';
                return;
            }
            this.slotError = '';
            try {
                await this.api(`/api/projects/${this.projectId}/slots`, 'POST', { name });
                await this.fetchProjectSlots();
                this.newSlotName = '';
            } catch (e) {
                this.slotError = e.message || 'Failed to add slot';
            }
        },

        async updateSlot(slotId, name) {
            const trimmed = (name || '').trim();
            if (!trimmed) return;
            await this.api(`/api/projects/${this.projectId}/slots/${slotId}`, 'PATCH', { name: trimmed });
            await this.fetchProjectSlots();
        },

        async deleteSlot(slotId) {
            if (!confirm('Delete this slot? Nodes using it will have their slot cleared.')) return;
            try {
                await this.api(`/api/projects/${this.projectId}/slots/${slotId}`, 'DELETE');
                await this.fetchProjectSlots();
            } catch (e) {
                this.slotError = e.message || 'Failed to delete slot';
            }
        },

        runLayout(opts = {}) {
            const cy = this.cy;
            const sort = buildStableRootSort(cy);
            const layout = cy.layout({
                name: 'dagre',
                rankDir: this.layoutDirection,
                nodeSep: 60,
                rankSep: 100,
                ranker: 'tight-tree',
                animate: true,
                animationDuration: 500,
                fit: !!opts.fit,
                sort
            });

            if (opts.fit) {
                layout.one('layoutstop', () => {
                    cy.animate({
                        fit: { padding: 50 },
                        duration: 300
                    });
                });
            }

            layout.run();
        }
    }));
};

if (window.Alpine) {
    registerGraph();
} else {
    document.addEventListener('alpine:init', registerGraph);
}

console.log('Boardtask graph persistence active');
