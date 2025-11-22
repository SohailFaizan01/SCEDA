# File: frontend/models.py

from dataclasses import dataclass, field
from typing import Optional, List, Dict, Any
from datetime import datetime

@dataclass
class Project:
    id: str
    name: str
    owner: str
    created_at: Optional[datetime] = None
    updated_at: Optional[datetime] = None
    
    @staticmethod
    def from_json(data: dict) -> 'Project':
        return Project(
            id=data['id'],
            name=data['name'],
            owner=data['owner'],
            created_at=data.get('created_at'),
            updated_at=data.get('updated_at')
        )
    
    def to_json(self) -> dict:
        return {
            'id': self.id,
            'name': self.name,
            'owner': self.owner,
        }


@dataclass
class IDEF0Block:
    id: str
    project_id: str
    name: str
    block_type: str
    signal_specs: Dict[str, Any]
    position_x: float
    position_y: float
    parent_block_id: Optional[str] = None
    children: List['IDEF0Block'] = field(default_factory=list)  # Built locally, not from JSON
    
    @staticmethod
    def from_json(data: dict) -> 'IDEF0Block':
        return IDEF0Block(
            id=data['id'],
            project_id=data['project_id'],
            name=data['name'],
            block_type=data['block_type'],
            signal_specs=data.get('signal_specs', {}),
            position_x=data.get('position_x', 0.0),
            position_y=data.get('position_y', 0.0),
            parent_block_id=data.get('parent_block_id')
        )
    
    def to_json(self) -> dict:
        return {
            'id': self.id,
            'project_id': self.project_id,
            'name': self.name,
            'block_type': self.block_type,
            'signal_specs': self.signal_specs,
            'position_x': self.position_x,
            'position_y': self.position_y,
            'parent_block_id': self.parent_block_id,
        }


@dataclass
class Component:
    id: str
    circuit_view_id: str
    component_type: str
    properties: Dict[str, Any]
    position_x: float
    position_y: float
    locked_by: Optional[str] = None
    lock_expires: Optional[datetime] = None
    
    @staticmethod
    def from_json(data: dict) -> 'Component':
        return Component(
            id=data['id'],
            circuit_view_id=data['circuit_view_id'],
            component_type=data['component_type'],
            properties=data.get('properties', {}),
            position_x=data.get('position_x', 0.0),
            position_y=data.get('position_y', 0.0),
            locked_by=data.get('locked_by'),
            lock_expires=data.get('lock_expires')
        )
    
    def to_json(self) -> dict:
        return {
            'id': self.id,
            'circuit_view_id': self.circuit_view_id,
            'component_type': self.component_type,
            'properties': self.properties,
            'position_x': self.position_x,
            'position_y': self.position_y,
            'locked_by': self.locked_by,
            'lock_expires': self.lock_expires,
        }


@dataclass
class CircuitView:
    id: str
    idef_block_id: str
    schematic_data: Dict[str, Any]
    spice_netlist: Optional[str] = None
    
    @staticmethod
    def from_json(data: dict) -> 'CircuitView':
        return CircuitView(
            id=data['id'],
            idef_block_id=data['idef_block_id'],
            schematic_data=data.get('schematic_data', {}),
            spice_netlist=data.get('spice_netlist')
        )
    
    def to_json(self) -> dict:
        return {
            'id': self.id,
            'idef_block_id': self.idef_block_id,
            'schematic_data': self.schematic_data,
            'spice_netlist': self.spice_netlist,
        }


@dataclass
class Net:
    id: str
    circuit_view_id: str
    name: str
    connected_pins: List[str]
    
    @staticmethod
    def from_json(data: dict) -> 'Net':
        return Net(
            id=data['id'],
            circuit_view_id=data['circuit_view_id'],
            name=data['name'],
            connected_pins=data.get('connected_pins', [])
        )
    
    def to_json(self) -> dict:
        return {
            'id': self.id,
            'circuit_view_id': self.circuit_view_id,
            'name': self.name,
            'connected_pins': self.connected_pins,
        }