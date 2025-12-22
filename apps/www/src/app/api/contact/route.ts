import { Resend } from 'resend';
import { NextResponse } from 'next/server';

const resend = new Resend(process.env.RESEND_API_KEY);

export async function POST(req: Request) {
  try {
    const body = await req.json();
    const { firstName, lastName, email, company, lookingFor } = body;

    // Validate required fields
    if (!email || !firstName || !lastName || !lookingFor) {
      return NextResponse.json(
        { error: 'Missing required fields' },
        { status: 400 }
      );
    }

    // Create or update the contact
    // Note: Contact properties (company_name, looking_for) should be
    // created manually in Resend dashboard or via a one-time setup script.
    // Attempting to create them on every request causes rate limit issues.

    // Build properties object with custom fields
    // Property keys must match exactly what's defined in Resend dashboard
    const properties: Record<string, string> = {};
    if (company) properties.company_name = company;
    if (lookingFor) properties.looking_for = lookingFor;

    let { data, error } = await resend.contacts.create({
      email,
      firstName,
      lastName,
      unsubscribed: false,
      // Pass custom properties as a Record<string, string>
      ...(Object.keys(properties).length > 0 && { properties }),
    });

    if (error) {
      // If contact already exists, update it instead
      const errorMessage = error.message?.toLowerCase() || '';
      const isDuplicate =
        errorMessage.includes('already exists') ||
        errorMessage.includes('duplicate') ||
        error.statusCode === 409;

      if (isDuplicate) {
        // Build properties object for update
        const updateProperties: Record<string, string> = {};
        if (company) updateProperties.company_name = company;
        if (lookingFor) updateProperties.looking_for = lookingFor;

        const { data: updateData, error: updateError } = await resend.contacts.update({
          email,
          firstName,
          lastName,
          unsubscribed: false,
          // Pass custom properties as a Record<string, string>
          ...(Object.keys(updateProperties).length > 0 && { properties: updateProperties }),
        });

        if (updateError) {
          console.error('Resend update error:', updateError);
          return NextResponse.json(
            { error: updateError.message || 'Failed to update contact' },
            { status: 500 }
          );
        }

        return NextResponse.json({ success: true, data: updateData });
      }

      console.error('Resend create error:', error);
      return NextResponse.json(
        { error: error.message || 'Failed to create contact' },
        { status: error.statusCode || 500 }
      );
    }

    return NextResponse.json({ success: true, data });
  } catch (error) {
    console.error('Unexpected error:', error);
    return NextResponse.json(
      { error: error instanceof Error ? error.message : 'Unknown error' },
      { status: 500 }
    );
  }
}
