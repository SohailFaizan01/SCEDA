# File: frontend/cache.py

import threading
import requests
from typing import Dict, List, Optional, Set
from models import Project, IDEF0Block, Component, CircuitView, Net


class ProjectCache:
    """Local cache with tree structure and debounced sync"""
    
    def __init__(self, base_url: str):
        self.base_url = base_url
        self.project: Optional[Project] = None
        
        # Flat storage (fast O(1) lookup)
        self.blocks: Dict[str, IDEF0Block] = {}
        self.components: Dict[str, Component] = {}
        self.circuit_views: Dict[str, CircuitView] = {}
        self.nets: Dict[str, Net] = {}
        
        # Tree structure (computed from flat)
        self.root_blocks: List[IDEF0Block] = []
        
        # Dirty tracking (needs sync to backend)
        self.dirty_blocks: Set[str] = set()
        self.dirty_components: Set[str] = set()
        
        # Debounce timers
        self.sync_timers: Dict[str, threading.Timer] = {}
        self.sync_delay = 0.5  # seconds
    
    def load_project(self, project_id: str):
        """Download project and all blocks from backend"""
        # Get project info
        response = requests.get(f"{self.base_url}/api/projects/{project_id}")
        if response.status_code == 200:
            data = response.json()
            if data['success']:
                self.project = Project.from_json(data['data'])
        
        # Get all blocks for project (flat list from backend)
        response = requests.get(f"{self.base_url}/api/projects/{project_id}/blocks")
        if response.status_code == 200:
            data = response.json()
            if data['success']:
                blocks_data = data['data']
                
                # Store in flat dict
                self.blocks = {
                    b['id']: IDEF0Block.from_json(b) 
                    for b in blocks_data
                }
                
                # Build tree structure
                self._build_tree()
    
    def _build_tree(self):
        """Build tree from flat blocks"""
        # Reset tree
        for block in self.blocks.values():
            block.children = []
        
        # Attach children to parents
        self.root_blocks = []
        for block in self.blocks.values():
            if block.parent_block_id:
                parent = self.blocks.get(block.parent_block_id)
                if parent:
                    parent.children.append(block)
            else:
                # No parent = root block
                self.root_blocks.append(block)
    
    def get_block(self, block_id: str) -> Optional[IDEF0Block]:
        """Get block by ID (O(1) lookup)"""
        return self.blocks.get(block_id)
    
    def get_children(self, block_id: str) -> List[IDEF0Block]:
        """Get children of a block (from tree structure)"""
        block = self.blocks.get(block_id)
        return block.children if block else []
    
    def update_block(self, block: IDEF0Block):
        """Update block in cache and schedule sync"""
        # Update cache immediately
        self.blocks[block.id] = block
        
        # Rebuild tree if parent changed
        # (You could optimize this to only update affected branches)
        self._build_tree()
        
        # Mark dirty and schedule sync
        self.dirty_blocks.add(block.id)
        self._schedule_sync_block(block)
    
    def _schedule_sync_block(self, block: IDEF0Block):
        """Schedule sync with debounce"""
        timer_key = f"block_{block.id}"
        
        # Cancel existing timer
        if timer_key in self.sync_timers:
            self.sync_timers[timer_key].cancel()
        
        # Schedule new sync
        timer = threading.Timer(
            self.sync_delay, 
            self._sync_block, 
            [block]
        )
        self.sync_timers[timer_key] = timer
        timer.start()
    
    def _sync_block(self, block: IDEF0Block):
        """Actually sync block to backend"""
        try:
            response = requests.put(
                f"{self.base_url}/api/blocks/{block.id}",
                json=block.to_json(),
                timeout=5
            )
            
            if response.status_code == 200:
                # Success - remove from dirty set
                self.dirty_blocks.discard(block.id)
                print(f"✓ Synced block {block.name}")
            else:
                print(f"✗ Failed to sync block {block.name}: {response.status_code}")
        
        except Exception as e:
            print(f"✗ Error syncing block {block.name}: {e}")
    
    def update_component(self, component: Component):
        """Update component in cache and schedule sync"""
        self.components[component.id] = component
        self.dirty_components.add(component.id)
        self._schedule_sync_component(component)
    
    def _schedule_sync_component(self, component: Component):
        """Schedule component sync with debounce"""
        timer_key = f"component_{component.id}"
        
        if timer_key in self.sync_timers:
            self.sync_timers[timer_key].cancel()
        
        timer = threading.Timer(
            self.sync_delay,
            self._sync_component,
            [component]
        )
        self.sync_timers[timer_key] = timer
        timer.start()
    
    def _sync_component(self, component: Component):
        """Actually sync component to backend"""
        try:
            response = requests.put(
                f"{self.base_url}/api/components/{component.id}",
                json=component.to_json(),
                timeout=5
            )
            
            if response.status_code == 200:
                self.dirty_components.discard(component.id)
                print(f"✓ Synced component {component.component_type}")
            else:
                print(f"✗ Failed to sync component: {response.status_code}")
        
        except Exception as e:
            print(f"✗ Error syncing component: {e}")
    
    def force_sync_all(self):
        """Force immediate sync of all dirty objects"""
        # Cancel all pending timers
        for timer in self.sync_timers.values():
            timer.cancel()
        self.sync_timers.clear()
        
        # Sync all dirty blocks
        for block_id in list(self.dirty_blocks):
            block = self.blocks.get(block_id)
            if block:
                self._sync_block(block)
        
        # Sync all dirty components
        for component_id in list(self.dirty_components):
            component = self.components.get(component_id)
            if component:
                self._sync_component(component)
    
    def has_unsaved_changes(self) -> bool:
        """Check if there are unsaved changes"""
        return bool(self.dirty_blocks or self.dirty_components)