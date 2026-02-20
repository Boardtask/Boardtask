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

const DONE_STATUS_ID = '01JSTATUS00000000DONE0000';

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
 * @param {{ selected: boolean, muted: boolean }} opts
 */
function buildNodeLabelHtml(data, opts) {
    const typeName = escapeHtml(data.node_type_name || 'Task');
    const typeColor = escapeHtml(data.node_type_color || '#4F46E5');
    const statusName = escapeHtml(data.status_name || '');
    const statusHtml = statusName ? `<div class="cy-node__status">${statusName}</div>` : '';
    const slotName = escapeHtml(data.slot_name || '');
    const slotHtml = slotName ? `<div class="cy-node__slot" title="${slotName}">${slotName}</div>` : '';
    const estimateStrRaw = formatEstimatedMinutes(data.estimated_minutes);
    const estimateStr = escapeHtml(estimateStrRaw);
    const estimateHtml = estimateStr ? `<div class="cy-node__estimate">${estimateStr}</div>` : '';
    const compactClass = (!estimateStrRaw && !data.status_name && !data.slot_name) ? ' cy-node--compact' : '';
    const mutedClass = (opts.muted) ? ' cy-node--muted' : '';
    const selectedClass = opts.selected ? ' cy-node--selected' : '';
    const label = escapeHtml(data.label ?? '');
    return `<div class="cy-node${compactClass}${mutedClass}${selectedClass}" style="border-color: ${typeColor}; border-left-color: ${typeColor};">
                                <div class="cy-node__header">
                                    <span class="cy-node__type" style="color: ${typeColor};">${typeName}</span>
                                    ${slotHtml}
                                </div>
                                <div class="cy-node__label">${label}</div>
                                <div class="cy-node__meta">${statusHtml}${estimateHtml}</div>
                            </div>`;
}

const registerGraph = () => {
    if (!window.Alpine) return;

    Alpine.store('projectFlash', { show: false, message: '' });

    if (Alpine.data('graph')) return;

    Alpine.data('graph', (projectId) => ({
        projectId: projectId,
        cy: null,
        selectedNodeIds: [],
        layoutDirection: 'TB',
        nodeTypeId: DEFAULTS.NODE_TYPE,
        nodeTypes: [],
        taskStatuses: [],
        projectSlots: [],
        editingNode: null, // { id, title, description, node_type_id, status_id, slot_id, estimated_amount, estimated_unit }
        saving: false,
        settingsOpen: false,
        highlightBlockedTodos: true,
        editingSlotId: null,
        editingSlotName: '',
        newSlotName: '',
        slotError: '',

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
                            'width': 2,
                            'line-color': '#C7D2FE',
                            'target-arrow-color': '#C7D2FE',
                            'target-arrow-shape': 'triangle',
                            'curve-style': 'bezier'
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
                            muted: this.highlightBlockedTodos && data.muted
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
                            muted: this.highlightBlockedTodos && data.muted
                        })
                    }
                ]);
            }

            this.cy.on('select', 'node', (evt) => {
                this.settingsOpen = false;

                const node = evt.target;
                const id = node.id();

                if (!this.selectedNodeIds.includes(id)) {
                    this.selectedNodeIds.push(id);
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

                if (this.selectedNodeIds.length > 2) {
                    const firstId = this.selectedNodeIds.shift();
                    this.cy.$id(firstId).unselect();
                }
                this.refreshNodeLabels();
            });

            this.cy.on('unselect', 'node', (evt) => {
                const id = evt.target.id();
                this.selectedNodeIds = this.selectedNodeIds.filter(nodeId => nodeId !== id);

                if (this.editingNode && this.editingNode.id === id) {
                    this.editingNode = null;
                }
                this.refreshNodeLabels();
            });

            this.cy.on('tap', (evt) => {
                if (evt.target === this.cy) {
                    this.cy.nodes().unselect();
                    this.selectedNodeIds = [];
                    this.editingNode = null;
                    this.refreshNodeLabels();
                }
            });

            await Promise.all([this.fetchNodeTypes(), this.fetchTaskStatuses(), this.fetchProjectSlots()]);
            await this.fetchGraph();
        },

        async fetchGraph() {
            try {
                const response = await fetch(`/api/projects/${this.projectId}/graph`);
                if (!response.ok) throw new Error('Failed to fetch graph');
                const data = await response.json();

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
                        return {
                            group: 'nodes',
                            data: {
                                id: n.id,
                                label: n.title,
                                description: n.description,
                                node_type_id: n.node_type_id,
                                node_type_name: type ? type.name : '',
                                node_type_color: type ? type.color : '#4F46E5',
                                status_id: n.status_id ?? DEFAULTS.STATUS_ID,
                                status_name: status ? status.name : 'To do',
                                slot_id: n.slot_id ?? '',
                                slot_name: slot ? slot.name : '',
                                estimated_minutes: n.estimated_minutes ?? null,
                                muted: !!muted
                            }
                        };
                    }),
                    ...data.edges.map(e => ({ group: 'edges', data: { source: e.parent_id, target: e.child_id } }))
                ];

                this.cy.elements().remove();
                this.cy.add(elements);
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
                this.cy.add({
                    group: 'nodes',
                    data: {
                        id: node.id,
                        label: node.title,
                        description: node.description,
                        node_type_id: node.node_type_id,
                        node_type_name: type ? type.name : '',
                        status_id: node.status_id ?? DEFAULTS.STATUS_ID,
                        status_name: status ? status.name : 'To do',
                        node_type_color: type ? type.color : '#4F46E5',
                        slot_id: node.slot_id ?? '',
                        slot_name: slot ? slot.name : '',
                        estimated_minutes: node.estimated_minutes ?? null,
                        muted: false
                    }
                });
                this.runLayout({ fit: true });
            } catch (error) {
                alert(`Error adding node: ${error.message}`);
            }
        },

        async addChildNode() {
            if (this.selectedNodeIds.length === 0) return;
            const parentId = this.selectedNodeIds[this.selectedNodeIds.length - 1];

            try {
                const title = DEFAULTS.NODE_TITLE;

                const node = await this.api(`/api/projects/${this.projectId}/nodes`, 'POST', {
                    node_type_id: this.nodeTypeId,
                    title: title,
                    description: ""
                });

                await this.api(`/api/projects/${this.projectId}/edges`, 'POST', {
                    parent_id: parentId,
                    child_id: node.id
                });

                const type = this.nodeTypes.find(t => t.id === node.node_type_id);
                const status = this.taskStatuses.find(s => s.id === (node.status_id ?? DEFAULTS.STATUS_ID));
                const slot = this.projectSlots.find(s => s.id === (node.slot_id || ''));
                const parentNode = this.cy.$id(parentId);
                const isParentRoot = parentNode.incomers().length === 0;
                const parentDone = (parentNode.data('status_id') || DEFAULTS.STATUS_ID) === DONE_STATUS_ID;
                const newNodeDone = (node.status_id ?? DEFAULTS.STATUS_ID) === DONE_STATUS_ID;
                const muted = !newNodeDone && !isParentRoot && !parentDone;
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
                            status_id: node.status_id ?? DEFAULTS.STATUS_ID,
                            status_name: status ? status.name : 'To do',
                            slot_id: node.slot_id ?? '',
                            slot_name: slot ? slot.name : '',
                            estimated_minutes: node.estimated_minutes ?? null,
                            muted: !!muted
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
                            status_id: node.status_id ?? DEFAULTS.STATUS_ID,
                            status_name: status ? status.name : 'To do',
                            slot_id: node.slot_id ?? '',
                            slot_name: slot ? slot.name : '',
                            estimated_minutes: node.estimated_minutes ?? null,
                            muted: false
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
            } catch (error) {
                alert(`Error adding parent node: ${error.message}`);
            }
        },

        async connectNodes() {
            if (this.selectedNodeIds.length !== 2) return;

            const sourceId = this.selectedNodeIds[0];
            const targetId = this.selectedNodeIds[1];

            try {
                await this.api(`/api/projects/${this.projectId}/edges`, 'POST', {
                    parent_id: sourceId,
                    child_id: targetId
                });

                this.cy.add({ group: 'edges', data: { source: sourceId, target: targetId } });
                this.runLayout();
                this.recomputeMutedForGraph();
            } catch (error) {
                alert(`Error connecting nodes: ${error.message}`);
            }
        },

        async disconnectNodes() {
            if (this.selectedNodeIds.length !== 2) return;

            const n1 = this.selectedNodeIds[0];
            const n2 = this.selectedNodeIds[1];

            try {
                // Try parent->child
                await this.api(`/api/projects/${this.projectId}/edges`, 'DELETE', {
                    parent_id: n1,
                    child_id: n2
                });

                this.cy.edges().filter(e => e.source().id() === n1 && e.target().id() === n2).remove();
                this.runLayout();
                this.recomputeMutedForGraph();
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
                    await this.api(`/api/projects/${this.projectId}/nodes/${node.id()}`, 'DELETE');
                    this.cy.remove(node);
                }
                this.selectedNodeIds = [];
                this.editingNode = null;
                this.runLayout();
            } catch (error) {
                alert(`Error removing node: ${error.message}`);
            }
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
                Alpine.store('projectFlash', { show: true, message: 'Save Success!' });
                setTimeout(() => {
                    Alpine.store('projectFlash', { show: false, message: '' });
                }, 3000);
            } catch (error) {
                alert(`Error saving node: ${error.message}`);
            } finally {
                this.saving = false;
            }
        },

        closeEditPanel() {
            if (this.editingNode) {
                this.cy.$id(this.editingNode.id).unselect();
            }
        },

        openSettings() {
            this.settingsOpen = true;
            this.editingNode = null;
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
            const layout = this.cy.layout({
                name: 'dagre',
                rankDir: this.layoutDirection,
                nodeSep: 60,
                rankSep: 100,
                ranker: 'tight-tree',
                animate: true,
                animationDuration: 500
            });

            if (opts.fit) {
                layout.one('layoutstop', () => {
                    this.cy.animate({
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
