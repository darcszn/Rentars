/**
 * Unit tests for property service.
 * Uses bun:test with module-level Supabase mock.
 */

import { describe, it, expect, beforeEach, mock } from 'bun:test';
import { mockProperties } from '../mocks/supabase.mock.data.js';

// ── Supabase + cache mock ─────────────────────────────────────────────────────

const mockFrom = mock((_: string) => ({}));
const mockSupabase = { from: mockFrom };
const supabaseMod = await import('../../src/config/supabase.js');
(supabaseMod as any).supabase = mockSupabase;

// Stub cache to be a no-op
const cacheMod = await import('../../src/services/cache.service.js');
(cacheMod as any).get = mock(async () => null);
(cacheMod as any).set = mock(async () => {});
(cacheMod as any).del = mock(async () => {});

import {
  getAllProperties,
  getPropertyById,
  createProperty,
  updateProperty,
  deleteProperty,
  searchProperties,
  type Property,
} from '../../src/services/property.service.js';

// ─────────────────────────────────────────────────────────────────────────────

describe('property.service', () => {
  beforeEach(() => {
    mockFrom.mockClear();
  });

  // ── getAllProperties ────────────────────────────────────────────────────────

  describe('getAllProperties', () => {
    it('should return a list of properties', async () => {
      mockFrom.mockImplementation(() => ({
        select: mock(() => ({
          order: mock(async () => ({ data: mockProperties, error: null })),
        })),
      }));

      const result = await getAllProperties();
      expect(result.success).toBe(true);
      expect(result.data).toEqual(mockProperties);
    });

    it('should handle database error', async () => {
      mockFrom.mockImplementation(() => ({
        select: mock(() => ({
          order: mock(async () => ({ data: null, error: { message: 'Database connection failed' } })),
        })),
      }));

      const result = await getAllProperties();
      expect(result.success).toBe(false);
      expect(result.error).toBe('Database connection failed');
    });
  });

  // ── getPropertyById ─────────────────────────────────────────────────────────

  describe('getPropertyById', () => {
    it('should return a property when found', async () => {
      const property = mockProperties[0];
      mockFrom.mockImplementation(() => ({
        select: mock(() => ({
          eq: mock(() => ({
            single: mock(async () => ({ data: property, error: null })),
          })),
        })),
      }));

      const result = await getPropertyById(property.id);
      expect(result.success).toBe(true);
      expect(result.data).toEqual(property);
    });

    it('should return error when property not found', async () => {
      mockFrom.mockImplementation(() => ({
        select: mock(() => ({
          eq: mock(() => ({
            single: mock(async () => ({ data: null, error: { message: 'No rows found' } })),
          })),
        })),
      }));

      const result = await getPropertyById('non-existent-id');
      expect(result.success).toBe(false);
      expect(result.error).toBe('Property not found');
    });

    it('should return error when ID is empty', async () => {
      const result = await getPropertyById('');
      expect(result.success).toBe(false);
      expect(result.error).toBe('Property ID is required');
    });
  });

  // ── createProperty ──────────────────────────────────────────────────────────

  describe('createProperty', () => {
    it('should create a property successfully', async () => {
      const newProperty: Partial<Property> = { title: 'New Property', price_per_night: 150, city: 'Paris' };
      const createdProperty = { id: 'new-id', ...newProperty };

      mockFrom.mockImplementation(() => ({
        insert: mock(() => ({
          select: mock(() => ({
            single: mock(async () => ({ data: createdProperty, error: null })),
          })),
        })),
      }));

      const result = await createProperty(newProperty);
      expect(result.success).toBe(true);
      expect(result.data?.title).toBe('New Property');
    });

    it('should return error when title is missing', async () => {
      const result = await createProperty({ price_per_night: 100 });
      expect(result.success).toBe(false);
      expect(result.error).toBe('Property title is required');
    });

    it('should handle database error during creation', async () => {
      mockFrom.mockImplementation(() => ({
        insert: mock(() => ({
          select: mock(() => ({
            single: mock(async () => ({ data: null, error: { message: 'Unique constraint violation' } })),
          })),
        })),
      }));

      const result = await createProperty({ title: 'Duplicate' });
      expect(result.success).toBe(false);
      expect(result.error).toBe('Unique constraint violation');
    });
  });

  // ── updateProperty ──────────────────────────────────────────────────────────

  describe('updateProperty', () => {
    it('should update a property successfully', async () => {
      const propertyId = mockProperties[0].id;
      const updatedProperty = { ...mockProperties[0], price_per_night: 200 };

      mockFrom.mockImplementation(() => ({
        update: mock(() => ({
          eq: mock(() => ({
            select: mock(() => ({
              single: mock(async () => ({ data: updatedProperty, error: null })),
            })),
          })),
        })),
      }));

      const result = await updateProperty(propertyId, { price_per_night: 200 });
      expect(result.success).toBe(true);
    });

    it('should return error when ID is missing', async () => {
      const result = await updateProperty('', { price_per_night: 200 });
      expect(result.success).toBe(false);
      expect(result.error).toBe('Property ID is required');
    });

    it('should return error when no fields provided', async () => {
      const result = await updateProperty('some-id', {});
      expect(result.success).toBe(false);
      expect(result.error).toBe('No fields provided for update');
    });
  });

  // ── deleteProperty ──────────────────────────────────────────────────────────

  describe('deleteProperty', () => {
    it('should delete a property successfully', async () => {
      mockFrom.mockImplementation(() => ({
        delete: mock(() => ({
          eq: mock(async () => ({ error: null })),
        })),
      }));

      const result = await deleteProperty('property-id');
      expect(result.success).toBe(true);
    });

    it('should return error when ID is missing', async () => {
      const result = await deleteProperty('');
      expect(result.success).toBe(false);
      expect(result.error).toBe('Property ID is required');
    });

    it('should handle database error during deletion', async () => {
      mockFrom.mockImplementation(() => ({
        delete: mock(() => ({
          eq: mock(async () => ({ error: { message: 'Row not found' } })),
        })),
      }));

      const result = await deleteProperty('non-existent');
      expect(result.success).toBe(false);
      expect(result.error).toBe('Row not found');
    });
  });

  // ── searchProperties ────────────────────────────────────────────────────────

  describe('searchProperties', () => {
    it('should search properties with city filter', async () => {
      const filtered = mockProperties.filter((p) => p.city === 'New York');
      mockFrom.mockImplementation(() => ({
        select: mock(() => ({
          ilike: mock(() => ({
            order: mock(async () => ({ data: filtered, error: null })),
          })),
        })),
      }));

      const result = await searchProperties({ city: 'New York' });
      expect(result.success).toBe(true);
      expect(result.data).toEqual(filtered);
    });

    it('should handle search error', async () => {
      mockFrom.mockImplementation(() => ({
        select: mock(() => ({
          order: mock(async () => ({ data: null, error: { message: 'Search failed' } })),
        })),
      }));

      const result = await searchProperties({});
      expect(result.success).toBe(false);
      expect(result.error).toBe('Search failed');
    });
  });
});
