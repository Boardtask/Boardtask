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
    NODE_TITLE: "New Node"
};

const registerGraph = () => {
    if (!window.Alpine) return;

    if (Alpine.data('graph')) return;

    Alpine.data('graph', (projectId) => ({
        projectId: projectId,
        cy: null,
        selectedNodeIds: [],
        layoutDirection: 'TB',
        nodeTypeId: DEFAULTS.NODE_TYPE,
        nodeTypes: [],
        editingNode: null, // { id: string, title: string, description: string, node_type_id: string }
        saving: false,
        saveSuccess: false,

        async init() {
            this.cy = cytoscape({
                container: this.$refs.canvas,
                boxSelectionEnabled: false,
                autounselectify: false,
                style: [
                    {
                        selector: 'node',
                        style: {
                            'width': 50,
                            'height': 50,
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
                        tpl: (data) => `<div class="cy-node"><div class="cy-node__label">${data.label}</div></div>`
                    },
                    {
                        query: 'node:selected',
                        halign: 'center',
                        valign: 'center',
                        halignBox: 'center',
                        valignBox: 'center',
                        tpl: (data) => `<div class="cy-node cy-node--selected"><div class="cy-node__label">${data.label}</div></div>`
                    }
                ]);
            }

            this.cy.on('select', 'node', (evt) => {
                const node = evt.target;
                const id = node.id();

                if (!this.selectedNodeIds.includes(id)) {
                    this.selectedNodeIds.push(id);
                }

                // Set editing node when a single node is selected or is the last selected
                this.editingNode = {
                    id: id,
                    title: node.data('label'),
                    description: node.data('description') || '',
                    node_type_id: node.data('node_type_id')
                };
                this.saveSuccess = false;

                if (this.selectedNodeIds.length > 2) {
                    const firstId = this.selectedNodeIds.shift();
                    this.cy.$id(firstId).unselect();
                }
            });

            this.cy.on('unselect', 'node', (evt) => {
                const id = evt.target.id();
                this.selectedNodeIds = this.selectedNodeIds.filter(nodeId => nodeId !== id);

                if (this.editingNode && this.editingNode.id === id) {
                    this.editingNode = null;
                }
            });

            this.cy.on('tap', (evt) => {
                if (evt.target === this.cy) {
                    this.cy.nodes().unselect();
                    this.selectedNodeIds = [];
                    this.editingNode = null;
                }
            });

            await this.fetchNodeTypes();
            await this.fetchGraph();
        },

        async fetchGraph() {
            try {
                const response = await fetch(`/api/projects/${this.projectId}/graph`);
                if (!response.ok) throw new Error('Failed to fetch graph');
                const data = await response.json();

                const elements = [
                    ...data.nodes.map(n => ({ group: 'nodes', data: { id: n.id, label: n.title, description: n.description, node_type_id: n.node_type_id } })),
                    ...data.edges.map(e => ({ group: 'edges', data: { source: e.parent_id, target: e.child_id } }))
                ];

                this.cy.elements().remove();
                this.cy.add(elements);
                this.runLayout();
            } catch (error) {
                console.error('Fetch error:', error);
                alert('Could not load graph data.');
            }
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

                this.cy.add({ group: 'nodes', data: { id: node.id, label: node.title, description: node.description, node_type_id: node.node_type_id } });
                this.runLayout();
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

                this.cy.add([
                    { group: 'nodes', data: { id: node.id, label: node.title, description: node.description, node_type_id: node.node_type_id } },
                    { group: 'edges', data: { source: parentId, target: node.id } }
                ]);
                this.runLayout();
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

                this.cy.add([
                    { group: 'nodes', data: { id: node.id, label: node.title, description: node.description, node_type_id: node.node_type_id } },
                    { group: 'edges', data: { source: node.id, target: childId } }
                ]);
                this.runLayout();
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
            this.saveSuccess = false;

            try {
                await this.api(`/api/projects/${this.projectId}/nodes/${this.editingNode.id}`, 'PATCH', {
                    title: this.editingNode.title,
                    description: this.editingNode.description,
                    node_type_id: this.editingNode.node_type_id
                });

                // Update Cytoscape node
                const cyNode = this.cy.$id(this.editingNode.id);
                cyNode.data('label', this.editingNode.title);
                cyNode.data('description', this.editingNode.description);
                cyNode.data('node_type_id', this.editingNode.node_type_id);

                this.saveSuccess = true;
                setTimeout(() => {
                    this.saveSuccess = false;
                }, 3000);
            } catch (error) {
                alert(`Error saving node: ${error.message}`);
            } finally {
                this.saving = false;
            }
        },

        runLayout() {
            const layout = this.cy.layout({
                name: 'dagre',
                rankDir: this.layoutDirection,
                nodeSep: 60,
                rankSep: 100,
                ranker: 'tight-tree',
                animate: true,
                animationDuration: 500
            });

            layout.one('layoutstop', () => {
                this.cy.animate({
                    fit: { padding: 50 },
                    duration: 300
                });
            });

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
