import { render, screen, fireEvent } from '@testing-library/react';
import { EndpointsEditor } from './EndpointsEditor';

describe('EndpointsEditor', () => {
  const mockOnChange = vi.fn();

  beforeEach(() => {
    mockOnChange.mockClear();
  });

  it('renders empty state with add button when no endpoints', () => {
    render(<EndpointsEditor value={{}} onChange={mockOnChange} />);
    expect(screen.getByText('No endpoints configured')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /add endpoint/i })).toBeInTheDocument();
  });

  it('adds new endpoint row when add button clicked', () => {
    render(<EndpointsEditor value={{}} onChange={mockOnChange} />);
    fireEvent.click(screen.getByRole('button', { name: /add endpoint/i }));
    expect(mockOnChange).toHaveBeenCalledWith({ default: '' });
  });

  it('displays existing endpoints', () => {
    render(<EndpointsEditor value={{ openai: 'https://api.openai.com/v1' }} onChange={mockOnChange} />);
    expect(screen.getByDisplayValue('https://api.openai.com/v1')).toBeInTheDocument();
  });

  it('removes endpoint when delete button clicked', () => {
    render(<EndpointsEditor value={{ openai: 'https://api.openai.com/v1' }} onChange={mockOnChange} />);
    fireEvent.click(screen.getByRole('button', { name: /remove/i }));
    expect(mockOnChange).toHaveBeenCalledWith({});
  });

  it('updates endpoint URL when changed', () => {
    render(<EndpointsEditor value={{ openai: 'https://api.openai.com/v1' }} onChange={mockOnChange} />);
    const input = screen.getByDisplayValue('https://api.openai.com/v1');
    fireEvent.change(input, { target: { value: 'https://new.url.com' } });
    expect(mockOnChange).toHaveBeenCalledWith({ openai: 'https://new.url.com' });
  });

  it('updates key when protocol dropdown changes', () => {
    render(<EndpointsEditor value={{ openai: 'https://api.openai.com/v1' }} onChange={mockOnChange} />);
    const select = screen.getByRole('combobox');
    fireEvent.change(select, { target: { value: 'anthropic' } });
    expect(mockOnChange).toHaveBeenCalledWith({ anthropic: 'https://api.openai.com/v1' });
  });
});