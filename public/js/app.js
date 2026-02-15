// Main JavaScript module for Boardtask

const registerGraph = () => {
    if (!window.Alpine) return;

    if (Alpine.data('graph')) return;

    Alpine.data('graph', () => ({
        cy: null,
        selectedNodeIds: [],
        layoutDirection: 'TB', // 'TB' for Top-Down, 'LR' for Left-to-Right

        init() {
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
                            'opacity': 0, // Hide the canvas node, let HTML show
                            'label': '' // Labels handled by HTML plugin
                        }
                    },
                    {
                        selector: 'edge',
                        style: {
                            'width': 2,
                            'line-color': '#C7D2FE', // Indigo-200
                            'target-arrow-color': '#C7D2FE',
                            'target-arrow-shape': 'triangle',
                            'curve-style': 'bezier'
                        }
                    }
                ],
                elements: [
                    { data: { id: 'root', label: 'Root' } }
                ],
                layout: {
                    name: 'dagre',
                    rankDir: this.layoutDirection,
                    nodeSep: 60,
                    rankSep: 100
                }
            });

            // Initialize HTML labels to allow standard browser CSS
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

            // Handle selection with 2-node limit
            this.cy.on('select', 'node', (evt) => {
                const id = evt.target.id();
                if (!this.selectedNodeIds.includes(id)) {
                    this.selectedNodeIds.push(id);
                }

                // Limit to 2 nodes (FIFO)
                if (this.selectedNodeIds.length > 2) {
                    const firstId = this.selectedNodeIds.shift();
                    this.cy.$id(firstId).unselect();
                }
            });

            this.cy.on('unselect', 'node', (evt) => {
                const id = evt.target.id();
                this.selectedNodeIds = this.selectedNodeIds.filter(nodeId => nodeId !== id);
            });

            this.cy.on('tap', (evt) => {
                if (evt.target === this.cy) {
                    this.cy.nodes().unselect();
                    this.selectedNodeIds = [];
                }
            });

            // Initial fit
            this.cy.ready(() => {
                this.runLayout();
            });
        },

        addNode() {
            const id = 'n' + (this.cy.nodes().length + 1);
            const label = 'Node ' + (this.cy.nodes().length + 1);

            this.cy.add({
                group: 'nodes',
                data: { id, label }
            });

            this.runLayout();
        },

        addChildNode() {
            if (this.selectedNodeIds.length === 0) return;
            const parentId = this.selectedNodeIds[this.selectedNodeIds.length - 1];

            const id = 'n' + (this.cy.nodes().length + 1);
            const label = 'Child ' + (this.cy.nodes().length + 1);

            this.cy.add([
                { group: 'nodes', data: { id, label } },
                { group: 'edges', data: { source: parentId, target: id } }
            ]);

            this.runLayout();
        },

        addParentNode() {
            if (this.selectedNodeIds.length === 0) return;
            const childId = this.selectedNodeIds[this.selectedNodeIds.length - 1];

            const id = 'n' + (this.cy.nodes().length + 1);
            const label = 'Parent ' + (this.cy.nodes().length + 1);

            this.cy.add([
                { group: 'nodes', data: { id, label } },
                { group: 'edges', data: { source: id, target: childId } }
            ]);

            this.runLayout();
        },

        connectNodes() {
            if (this.selectedNodeIds.length !== 2) return;

            const sourceId = this.selectedNodeIds[0];
            const targetId = this.selectedNodeIds[1];

            const source = this.cy.$id(sourceId);
            const target = this.cy.$id(targetId);

            if (target.successors().contains(source)) {
                alert("Cannot connect: This would create a cycle!");
                return;
            }

            if (source.edgesTo(target).length > 0) {
                alert("Nodes are already connected.");
                return;
            }

            this.cy.add({
                group: 'edges',
                data: { source: sourceId, target: targetId }
            });

            this.runLayout();
        },

        disconnectNodes() {
            if (this.selectedNodeIds.length !== 2) return;

            const n1 = this.cy.$id(this.selectedNodeIds[0]);
            const n2 = this.cy.$id(this.selectedNodeIds[1]);

            // Find edges in both directions
            const edges = n1.edgesWith(n2);

            if (edges.length === 0) {
                alert("No connection exists between these two nodes.");
                return;
            }

            this.cy.remove(edges);
            this.runLayout();
        },

        toggleDirection() {
            this.layoutDirection = this.layoutDirection === 'TB' ? 'LR' : 'TB';
            this.runLayout();
        },

        removeNode() {
            const nodes = this.cy.nodes(':selected');
            if (nodes.length > 0) {
                this.cy.remove(nodes);
                this.selectedNodeIds = [];
            } else {
                const allNodes = this.cy.nodes();
                if (allNodes.length > 1) {
                    this.cy.remove(allNodes.last());
                }
            }
            this.runLayout();
        },

        runLayout() {
            const layout = this.cy.layout({
                name: 'dagre',
                rankDir: this.layoutDirection,
                // Refinements for better stacking
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

console.log('Boardtask app improved with Disconnect feature');
